
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
fn particle_rectangle_collision(i: usize,
    particles: &ParticleVec,
    rect_spec: &RectSpec,
    mut force: DVec3,
    mut torque: DVec3,
    settings: &SimulationSettings)->(DVec3,DVec3){

    // Extract simulation parameters
    let (stiffness, damping, mu_opt) = match &settings.model {
        SimulationModel::Solid(p) => (p.stiffness, p.damping, None),
        SimulationModel::SolidFriction(p) => (p.stiffness, p.damping, Some(p.mu)),
        _ => panic!("Unsupported model for granular collision"),
    };
    
    let particle_pos = particles.position[i];
    let radius = particles.radius[i];

    // Transform particle position into the rectangle's local coordinate system
    let rect_center = rect_spec.center;
    let half_size = rect_spec.half_size;
    let orientation_quat = glam::DQuat::from(rect_spec.orientation);
    let orientation_inv = orientation_quat.inverse();

    let to_particle = particle_pos - rect_center;
    let local_pos = orientation_inv * to_particle;

    // Find the closest point on the local box surface
    let clamped_local = DVec3::new(
    local_pos.x.clamp(-half_size.x, half_size.x),
    local_pos.y.clamp(-half_size.y, half_size.y),
    local_pos.z.clamp(0.0, 0.0)
    );

    let local_delta = local_pos - clamped_local;
    let dist_sq = local_delta.length_squared();

    // Check for collision overlap
    if dist_sq < radius * radius && dist_sq > 1e-18 {
        let dist = dist_sq.sqrt();
        
        // Normal pointing from rectangle surface to the particle (in world space)
        let local_normal = local_delta / dist;
        let normal = orientation_quat * local_normal;
        let overlap = radius - dist;

        // Calculate Rectangle's Surface Velocity at the Contact Point
        let r_rect = orientation_quat * clamped_local;
        let rect_surface_vel = rect_spec.velocity + rect_spec.omega.cross(r_rect);

        // Relative velocity: Particle velocity minus moving rectangle surface velocity
        let rel_vel = particles.velocity[i] - rect_surface_vel;
        let normal_vel = rel_vel.dot(normal);

        let f_normal_mag = (stiffness * overlap - damping * normal_vel).max(0.0);
        let f_normal_vec = normal * f_normal_mag;

        // Friction if applicable
        if let Some(mu) = mu_opt {
            let r_particle = normal * -radius; // Vector from particle center to contact point
            
            // Total tangential relative velocity (including particle rotation)
            let v_tang = rel_vel - (rel_vel.dot(normal) * normal)
                         + particles.omega[i].cross(r_particle);

            if v_tang.length_squared() > 1e-18 {
                let f_t_ideal = v_tang * -damping;
                let limit = mu * f_normal_mag;

                let f_t_mag_sq = f_t_ideal.length_squared();
                let f_t_vec = if f_t_mag_sq > limit * limit {
                    f_t_ideal * (limit / f_t_mag_sq.sqrt())
                } else {
                    f_t_ideal
                };

                // Apply friction forces and torques to the particle
                force += f_t_vec;
                torque += r_particle.cross(f_t_vec);
            }
        }

        // Apply Normal Force to the particle
        force += f_normal_vec;
    }

    (force, torque)
}
