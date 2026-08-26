
use glam::DVec3;

use crate::md_sim::particle::SimulationModel;
use crate::md_sim::{ObjectSpec, ParticleVec, SimulationSettings, SurfaceKinematics};


/// Computes contact forces and torques arising from collisions between a particle and simulation objects.
///
/// This acts as a dispatcher function that inspects the object specification variant 
/// (e.g., rectangles, triangles) and delegates to generic surface collision solver.
///
/// # Arguments
///
/// * `i` - Index of the particle being tested for collisions.
/// * `particles` - Reference to the particle state buffers (positions, velocities, radii, etc.).
/// * `object_spec` - Specification of the geometric objects present in the simulation.
/// * `force` - Accumulated incoming force vector for the particle.
/// * `torque` - Accumulated incoming torque vector for the particle.
/// * `settings` - Global simulation parameters containing material models (stiffness, damping, friction).
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
    settings: &SimulationSettings,
) -> (DVec3, DVec3) {
    match object_spec {
        ObjectSpec::Rectangle(rect) => {
            (force, torque) = particle_contact_response(i, particles, rect, force, torque, settings);
        }
        ObjectSpec::Triangle(tri) => {
            (force, torque) = particle_contact_response(i, particles, tri, force, torque, settings);
        }
        _ => {}
    }

    (force, torque)
}

/// Computes the linear force and rotational torque exerted on a particle 
/// colliding with a moving rigid surface (`SurfaceKinematics`).
///
/// This function uses a viscoelastic spring-dashpot contact model in the normal 
/// direction and a viscous-damping Coulomb friction model in the tangential direction.
///
/// # Mathematical Model
///
/// 1. **Overlap & Geometry**:
///    - $\mathbf{\delta} = \mathbf{p}_{\text{particle}} - \mathbf{p}_{\text{closest}}$
///    - $\text{overlap} = R - \|\mathbf{\delta}\|$
///    - $\mathbf{n} = \frac{\mathbf{\delta}}{\|\mathbf{\delta}\|}$ (unit vector towards particle)
///
/// 2. **Relative Velocity**:
///    - Particle contact point: $\mathbf{r}_{\text{particle}} = -\mathbf{n} R$
///    - Surface velocity at contact: $\mathbf{v}_{\text{surface}} = \text{surface.velocity\_at\_point}(\mathbf{p}_{\text{closest}})$
///    - Particle contact velocity: $\mathbf{v}_{\text{particle\_contact}} = \mathbf{v}_{\text{particle}} + \boldsymbol{\omega}_{\text{particle}} \times \mathbf{r}_{\text{particle}}$
///    - Relative velocity: $\mathbf{v}_{\text{rel}} = \mathbf{v}_{\text{particle\_contact}} - \mathbf{v}_{\text{surface}}$
///
/// 3. **Normal Force ($F_n$)**:
///    $$\mathbf{F}_n = \max(0, k_n \cdot \text{overlap} - \gamma_n (\mathbf{v}_{\text{rel}} \cdot \mathbf{n})) \mathbf{n}$$
///
/// 4. **Tangential Friction Force ($F_t$)**:
///    - Tangential velocity: $\mathbf{v}_{\text{tang}} = \mathbf{v}_{\text{rel}} - (\mathbf{v}_{\text{rel}} \cdot \mathbf{n})\mathbf{n}$
///    - Clamped by Coulomb limit: $\|\mathbf{F}_t\| \le \mu \|\mathbf{F}_n\|$
///
/// 5. **Induced Torque ($\boldsymbol{\tau}$)**:
///    $$\boldsymbol{\tau} = \mathbf{r}_{\text{particle}} \times \mathbf{F}_t$$
///
/// # Arguments
///
/// * `i` - Index of the active particle within the `ParticleVec` container.
/// * `particles` - Read-only reference to particle storage vectors (positions, velocities, radii, etc.).
/// * `surface` - Reference to any geometry implementing [`SurfaceKinematics`].
/// * `force` - Accumulator for total force applied to particle `i`. Modified and returned.
/// * `torque` - Accumulator for total torque applied to particle `i`. Modified and returned.
/// * `settings` - Simulation parameters containing contact stiffness, damping, and friction coefficients.
///
/// # Returns
///
/// * `(DVec3, DVec3)` - Updated `(force, torque)` tuple for particle `i`.
///
/// # Panics
///
/// Panics if `settings.model` is not variant [`SimulationModel::Frictional`].
pub (crate) fn particle_contact_response<S: SurfaceKinematics>(
    i: usize,
    particles: &ParticleVec,
    surface: &S,
    mut force: DVec3,
    mut torque: DVec3,
    settings: &SimulationSettings,
) -> (DVec3, DVec3) {
    //Ignore if not a collision ptype
    if !settings.collision_ptypes.contains(&(particles.ptype[i] as u8)){
        return (force, torque)
    }

    let (pl_stiffness, pl_damping, pl_mu) = if let SimulationModel::Frictional(p) = &settings.model {
        (p.plane_stiffness, p.plane_damping, p.plane_mu)
    } else {
        panic!("Unsupported model for granular collision");
    };

    let particle_pos = particles.position[i];
    let particle_vel = particles.velocity[i];
    let particle_omega = particles.omega[i];
    let radius = particles.radius[i];

    let closest_point = surface.closest_point(particle_pos);
    let delta = particle_pos - closest_point;
    let dist_sq = delta.length_squared();

    // Check overlap
    if dist_sq < radius * radius && dist_sq > 1e-18 {
        let dist = dist_sq.sqrt();
        let overlap = radius - dist;
        let normal = delta / dist; // Surface normal towards particle

        // Query surface velocity directly from Option 1 trait implementation
        let surface_vel = surface.velocity_at_point(closest_point);

        // Particle contact point velocity. r_particle is particle centre to contact point and not particle radius for torque calc.
        let r_particle = -normal * dist;
        let particle_contact_vel = particle_vel + particle_omega.cross(r_particle);

        // Relative velocity
        let rel_vel = particle_contact_vel - surface_vel;
        let normal_vel = rel_vel.dot(normal);

        // Normal force (spring-dashpot)
        let f_normal_mag = (pl_stiffness * overlap - pl_damping * normal_vel).max(0.0);
        let f_normal_vec = normal * f_normal_mag;

        // Friction force
        let v_tang = rel_vel - normal_vel * normal;
        let mut f_friction_vec = DVec3::ZERO;

        if v_tang.length_squared() > 1e-18 {
            let f_t_ideal = -v_tang * pl_damping;
            let limit = pl_mu * f_normal_mag;
            let f_t_mag_sq = f_t_ideal.length_squared();

            f_friction_vec = if f_t_mag_sq > limit * limit {
                f_t_ideal * (limit / f_t_mag_sq.sqrt())
            } else {
                f_t_ideal
            };
        }

        force += f_normal_vec + f_friction_vec;
        torque += r_particle.cross(f_friction_vec);
    }

    (force, torque)
}

