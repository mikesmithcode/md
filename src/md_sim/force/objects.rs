
use glam::DVec3;

use crate::md_sim::particle::SimulationModel;
use crate::md_sim::{RectSpec, ObjectSpec, ParticleVec, SimulationSettings};


/// Computes contact forces and torques arising from collisions between a particle and simulation objects.
///
/// This acts as a dispatcher function that inspects the object specification variant 
/// (e.g., rectangles, triangles) and delegates to the appropriate geometry collision solver.
/// Currently only implements Rectangles.
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
    settings: &SimulationSettings
) -> (DVec3, DVec3) {
    

    // Extract rectangle properties
    match object_spec {
        ObjectSpec::Rectangle(r) => {
            let (f, t) = particle_rectangle_collision(i, particles, r, force, torque, settings);
            force = f;
            torque = t;
        },
        _ => return (force, torque),
    };

    (force, torque)
}

// Calculates contact mechanics (normal forces and optional Coulomb/viscous friction) 
// between a spherical particle and a rotating/translating 3D rectangular object.
//
// # Arguments
//
// * `i` - Index of the colliding particle.
// * `particles` - Reference to the particle state buffers.
// * `rect_spec` - Geometry, position, orientation, and kinematic properties of the rectangle.
// * `force` - Accumulated incoming force vector.
// * `torque` - Accumulated incoming torque vector.
// * `settings` - Global simulation parameters defining the contact model (`Solid` or `SolidFriction`).
//
// # Returns
//
// * `(DVec3, DVec3)` - The force and torque contributions from the rectangle-particle collision.
fn particle_rectangle_collision(
    i: usize,
    particles: &ParticleVec,
    rect_spec: &RectSpec,
    mut force: DVec3,
    mut torque: DVec3,
    settings: &SimulationSettings,
) -> (DVec3, DVec3) {
    // Extract simulation parameters
    let (pl_stiffness, pl_damping, pl_mu) = match &settings.model {
        SimulationModel::Frictional(p) => (p.plane_stiffness, p.plane_damping, p.plane_mu),
        _ => panic!("Unsupported model for granular collision"),
    };
    
    let particle_pos = particles.position[i];
    let particle_vel = particles.velocity[i];
    let particle_omega = particles.omega[i];
    let radius = particles.radius[i];

    // Rectangle kinematics & pose
    let rect_center = rect_spec.center;
    let orientation_inv = rect_spec.orientation.inverse();

    // Transform to rectangle's local coords. Centre of rectangle in local coords is (0,0,0) and normal along +z.
    let to_particle = particle_pos - rect_center;
    let local_pos = orientation_inv * to_particle;

    // 2. Find the closest point on the rectangle surface 
    // half_size is a DVec2 (x = half_width, y = half_height), and Z is flat on the plane (0.0)
    let clamped_local = DVec3::new(
        local_pos.x.clamp(-rect_spec.half_size.x, rect_spec.half_size.x),
        local_pos.y.clamp(-rect_spec.half_size.y, rect_spec.half_size.y),
        0.0 // The plane surface lies at local z = 0
    );

    // Find distance between closest point and particle centre
    let local_delta = local_pos - clamped_local;
    let dist_sq = local_delta.length_squared();

    // Check for collision overlap (&& dist_sq > 1e-18)
    if dist_sq < radius * radius  {
        let dist = dist_sq.sqrt();
        let overlap = radius - dist;

        // Surface normal pointing from the rectangle surface to the particle (in world space)
        let local_normal = local_delta / dist;
        let normal = rect_spec.orientation * local_normal;
        

        // 3. Compute Rectangle's Surface Velocity at the exact contact point in world space
        // r_rect is the vector from the rectangle center to the contact point in world coordinates
        let clamped_global_offset = rect_spec.orientation * clamped_local;
        let rect_surface_vel = rect_spec.velocity + rect_spec.omega.cross(clamped_global_offset);

        // 4. Contact point vector relative to the sphere's center
        let r_particle = -normal * radius;

        // Total velocity of the particle's contact point (translation + spin)
        let particle_contact_vel = particle_vel + particle_omega.cross(r_particle);

        // Relative velocity between particle contact point and rectangle surface velocity
        let rel_vel = particle_contact_vel - rect_surface_vel;
        let normal_vel = rel_vel.dot(normal);

        // Normal force (spring-dashpot model, resisting closure)
        let f_normal_mag = (pl_stiffness * overlap - pl_damping * normal_vel).max(0.0);
        let f_normal_vec = normal * f_normal_mag;

        // 5. Tangential (Frictional) relative velocity
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

        // Accumulate forces and torques acting on the sphere
        force += f_normal_vec + f_friction_vec;
        torque += r_particle.cross(f_friction_vec);
    }

    (force, torque)
}
