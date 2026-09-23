
use glam::DVec3;

use crate::md_sim::force::pairwise::CollisionParams;
use crate::md_sim::{ObjectSpec, ParticleVec, SurfaceKinematics};
use crate::md_sim::force::common::compute_contact_force_and_torque;
use crate::md_sim::SimulationSettings;


/// Computes contact forces and torques arising from collisions between a particle and simulation objects.
///
/// This acts as a dispatcher function that inspects the object specification variant 
/// (e.g., rectangles, triangles) and delegates to the generic surface collision solver.
///
/// # Arguments
///
/// * `i` - Index of the particle being tested for collisions.
/// * `particles` - Reference to the particle state buffers (positions, velocities, radii, etc.).
/// * `object_spec` - Specification of the geometric objects present in the simulation.
/// * `force` - Accumulated incoming force vector for the particle.
/// * `torque` - Accumulated incoming torque vector for the particle.
/// * `model` - Collision parameters containing precomputed material properties and friction coefficients.
/// * `settings` - Simulation settings containing the O(1) collision mask and global configuration parameters.
///
/// # Returns
///
/// * `(DVec3, DVec3)` - The updated force and torque vectors including object interaction contributions.
pub fn add_particle_object_collision(
    i: usize,
    particles: &ParticleVec,
    object_spec: &ObjectSpec,
    mut force: DVec3,
    mut torque: DVec3,
    model: &CollisionParams,
    settings: &SimulationSettings,
) -> (DVec3, DVec3) {
    match object_spec {
        ObjectSpec::Rectangle(rect) => {
            (force, torque) = particle_contact_response(
                i, particles, rect, force, torque, model, settings
            );
        }
        ObjectSpec::Triangle(tri) => {
            (force, torque) = particle_contact_response(
                i, particles, tri, force, torque, model, settings
            );
        }
        _ => {}
    }

    (force, torque)
}
/// Computes the linear force and rotational torque exerted on a particle
/// colliding with a moving rigid surface (SurfaceKinematics).
/// 
/// This function uses a viscoelastic spring-dashpot contact model in the normal
/// direction and a viscous-damping Coulomb friction model in the tangential direction.
/// 
/// # Mathematical Model
/// 
/// 1. Overlap & Geometry:
///    - $\mathbf{\delta} = \mathbf{p}_{\text{particle}} - \mathbf{p}_{\text{closest}}$ (vector from surface closest point to particle center)
///    - $\text{dist} = \Vert{}\mathbf{\delta}\Vert{}$, $\text{overlap} = R - \text{dist}$
///    - $\mathbf{n} = \frac{\mathbf{\delta}}{\text{dist}}$ (unit vector pointing strictly outward from the surface toward the particle)
/// 
/// 2. Relative Velocity:
///    - Contact point offset: $\mathbf{r}_{\text{particle}} = -\mathbf{n} \cdot \text{dist}$ (vector from particle center of mass to the contact point)
///    - Surface velocity at contact: $\mathbf{v}_{\text{surface}} = \text{surface.velocity\_at\_point}(\mathbf{p}_{\text{closest}})$
///    - Particle contact point velocity: $\mathbf{v}_{\text{particle\_contact}} = \mathbf{v}_{\text{particle}} + \boldsymbol{\omega}_{\text{particle}} \times \mathbf{r}_{\text{particle}}$
///    - Relative velocity: $\mathbf{v}_{\text{rel}} = \mathbf{v}_{\text{particle\_contact}} - \mathbf{v}_{\text{surface}}$
///    - Normal velocity component: $v_n = \mathbf{v}_{\text{rel}} \cdot \mathbf{n}$ (negative during compression, positive during separation)
/// 
/// 3. Normal Force ($\mathbf{F}_n$):
///    - Elastic component: $F_{\text{elastic}} = k_n \cdot \text{overlap}$
///    - Viscous damping component: $F_{\text{damping}} = \gamma_n \cdot v_n$
///    - Clamped magnitude: $F_{n,\text{mag}} = \max\left(0, F_{\text{elastic}} - F_{\text{damping}}\right)$
///    - Vector normal force: $\mathbf{F}_n = F_{n,\text{mag}} \mathbf{n}$ (pointing outward from the surface)
/// 
/// 4. Tangential Friction Force ($\mathbf{F}_t$):
///    - Tangential relative velocity: $\mathbf{v}_{\text{tang}} = \mathbf{v}_{\text{rel}} - v_n \mathbf{n}$
///    - Ideal viscous friction: $\mathbf{F}_{t,\text{ideal}} = -\gamma_n \mathbf{v}_{\text{tang}}$
///    - Coulomb friction limit: $F_{\text{limit}} = \mu F_{n,\text{mag}}$
///    - Clamped friction vector: $\mathbf{F}_t = \min\left(1, \frac{F_{\text{limit}}}{\Vert{}\mathbf{F}_{t,\text{ideal}}\Vert{}}\right) \mathbf{F}_{t,\text{ideal}}$
/// 
/// 5. Induced Torque ($\boldsymbol{\tau}$):
///    $$\boldsymbol{\tau} = \mathbf{r}_{\text{particle}} \times \mathbf{F}_t$$
/// 
/// # Arguments
/// 
/// * `i` - Index of the active particle within the `ParticleVec` container.
/// * `particles` - Read-only reference to particle storage vectors (positions, velocities, radii, etc.).
/// * `surface` - Reference to any geometry implementing `[SurfaceKinematics]`.
/// * `force` - Accumulator for total force applied to particle `i`. Modified and returned.
/// * `torque` - Accumulator for total torque applied to particle `i`. Modified and returned.
/// * `model` - Collision parameters containing precomputed material properties and friction coefficients.
/// * `settings` - Simulation settings containing the $O(1)$ collision mask.
/// 
/// # Returns
/// 
/// * `(DVec3, DVec3)` - Updated (force, torque) tuple for particle `i`.
pub(crate) fn particle_contact_response<S: SurfaceKinematics>(
    i: usize,
    particles: &ParticleVec,
    surface: &S,
    mut force: DVec3,
    mut torque: DVec3,
    model: &CollisionParams,
    settings: &SimulationSettings, // Pass settings directly instead of raw pieces
) -> (DVec3, DVec3) { 
    // O(1) type check using settings mask
    if !settings.collision_mask[particles.ptype[i]] {
        return (force, torque);
    }

    let particle_pos = particles.position[i];
    let particle_vel = particles.velocity[i];
    let particle_omega = particles.omega[i];
    let radius = particles.radius[i];

    let closest_point = surface.closest_point(particle_pos);
    let delta = particle_pos - closest_point;
    let dist_sq = delta.length_squared();

    if dist_sq < radius * radius && dist_sq > 1e-18 {
        let dist = dist_sq.sqrt();
        let overlap = radius - dist;
        let normal = delta / dist; 

        let m_eff = particles.mass[i];      

        // Using precomputed invariant terms directly from model
        let eff_stiffness = (4.0 / 3.0) * model.plane_e_star * radius.sqrt();
        let eff_damping = 2.0 * model.plane_beta * (m_eff * eff_stiffness).sqrt();

        let surface_vel = surface.velocity_at_point(closest_point);
        let r_particle = -normal * dist;
        let particle_contact_vel = particle_vel + particle_omega.cross(r_particle);
        let rel_vel = particle_contact_vel - surface_vel;

        let (contact_force, contact_torque) = compute_contact_force_and_torque(
            overlap, normal, r_particle, rel_vel, eff_stiffness, eff_damping, model.plane_mu
        );
        
        force += contact_force;
        torque += contact_torque;
    }

    (force, torque)
}
