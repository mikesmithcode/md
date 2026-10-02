
use crate::md_sim::force::pairwise::CollisionParams;
use crate::md_sim::{ObjectSpec, ParticleVec, SurfaceKinematics};
use crate::md_sim::force::contact::{Contact};
use crate::md_sim::SimulationSettings;


/// Dispatches collision checks between a particle and simulation objects 
/// (e.g., rectangles, triangles), returning a `Contact` struct if a collision occurs.
///
/// # Arguments
///
/// * `i` - Index of the particle being tested for collisions.
/// * `particles` - Reference to the particle state buffers (positions, velocities, radii, etc.).
/// * `object_spec` - Specification of the geometric objects present in the simulation.
/// * `model` - Collision parameters containing precomputed material properties and friction coefficients.
/// * `settings` - Simulation settings containing the $O(1)$ collision mask and global configuration parameters.
///
/// # Returns
///
/// * `Some(Contact)` containing resolved collision geometry and relative kinematics if overlapping.
/// * `None` otherwise.
pub fn check_surface_contact(
    i: usize,
    particles: &ParticleVec,
    object_spec: &ObjectSpec,
    model: &CollisionParams,
    settings: &SimulationSettings,
) -> Option<Contact> {
    match object_spec {
        ObjectSpec::Rectangle(rect) => {
            surface_contact(
                i, particles, rect, model, settings
            )
        }
        ObjectSpec::Triangle(tri) => {
            surface_contact(
                i, particles, tri, model, settings
            )
        }
        _ => None
    }
}
/// Computes collision geometry, relative kinematics, and effective contact properties 
/// for a particle colliding with a moving rigid surface (`SurfaceKinematics`).
/// 
/// # Mathematical Model & Processing Steps
/// 
/// 1. **Collision Mask Filtering**:
///    - Exits early if particle $i$'s type is excluded by `settings.collision_mask`.
/// 2. **Geometry & Overlap**:
///    - Closest point on surface: $\mathbf{p}_{\text{closest}} = \text{surface.closest\_point}(\mathbf{p}_i)$
///    - Position delta: $\boldsymbol{\delta} = \mathbf{p}_i - \mathbf{p}_{\text{closest}}$ (pointing from surface to particle)
///    - Center-to-center distance: $\text{dist} = \Vert{}\boldsymbol{\delta}\Vert{}$
///    - Overlap: $\text{overlap} = R_i - \text{dist}$
///    - Unit normal vector: $\mathbf{n} = \frac{\boldsymbol{\delta}}{\text{dist}}$
/// 3. **Effective Properties & Hertzian Contact Parameters**:
///    - Reduced mass: $m_{\text{eff}} = m_i$
///    - Contact stiffness ($k_n$): $k_n = \frac{4}{3} E^*_{\text{plane}} \sqrt{R_i}$
///    - Damping coefficient ($\gamma_n$): $\gamma_n = 2 \beta_{\text{plane}} \sqrt{m_{\text{eff}} k_n}$
/// 4. **Kinematics & Contact Points**:
///    - Contact offset vector: $\mathbf{r}_{\text{particle}} = -\mathbf{n} \cdot \text{dist}$
///    - Relative velocity at contact: $\mathbf{v}_{\text{rel}} = (\mathbf{v}_i + \boldsymbol{\omega}_i \times \mathbf{r}_{\text{particle}}) - \mathbf{v}_{\text{surface}}$
/// 
/// # Arguments
/// 
/// * `i` - Index of the active particle within the `ParticleVec` container.
/// * `particles` - Read-only reference to particle storage vectors.
/// * `surface` - Reference to any geometry implementing `SurfaceKinematics`.
/// * `model` - Collision parameters containing plane elasticity, damping, and friction coefficients.
/// * `settings` - Simulation settings containing the $O(1)$ collision mask.
/// 
/// # Returns
/// 
/// * `Some(Contact)` containing the resolved geometry, kinematics, and material properties if overlapping.
/// * `None` if separated or filtered out by collision masks.
pub(crate) fn surface_contact<S: SurfaceKinematics>(
    i: usize,
    particles: &ParticleVec,
    surface: &S,
    model: &CollisionParams,
    settings: &SimulationSettings, // Pass settings directly instead of raw pieces
) -> Option<Contact> { 
    // O(1) type check using settings mask
    if !settings.collision_mask[particles.ptype[i]] {
        return None;
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

        let contact = Contact{overlap,normal,r_contact: r_particle,rel_vel, eff_stiffness, eff_damping, mu: model.plane_mu};
        Some(contact)
        
    }
    else{
        None
    }
}
