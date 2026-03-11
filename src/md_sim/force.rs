//! Force functions for use in the implementation of update_forces()
//! 
//! Your simulation should implement the Forces trait on a unit struct. This involves
//! defining the function update_forces(). You can call the forces functions in this module
//! inside update_forces or define your own.

use glam::DVec3;

use itertools::izip;
use std::f64::consts::PI;

use crate::CollisionParams;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;

pub trait Forces{
    fn update_forces(&self, forces: &mut [DVec3], particles: &ParticleVec, settings: &SimulationSettings);

}

/// Set all forces of particular ptype to zero
pub fn zero_forces_ptype(forces: &mut [DVec3], particles: &ParticleVec, ptype: usize){
    let n=particles.len();
    //set all forces to zero for immobile particles
    for k in 0..n{
        if particles.ptype[k] == ptype{
            forces[k] = DVec3::ZERO;
        }
    }
}

///Add the weight
/// 
///assumes g acts in -z direction. If in a fluid your inv_mass particle
///attribute should take into account the relative density.
pub fn add_weight(forces: &mut [DVec3], particles: &ParticleVec) {
    let gravity = -9.81;

    for (force, &inv_mass) in izip!(forces.iter_mut(), particles.inv_mass.iter()) {
        if inv_mass*inv_mass > 0.0 {
            let weight = gravity / inv_mass;
            force.z += weight;
        }
    }
}

///Add viscous drag
/// 
/// assumes these are spheres and that drag is proportional to velocity
pub fn add_viscous_drag(forces: &mut [DVec3], particles: &ParticleVec, viscosity: f64){
    for (force, &vel, &rad) in izip!(forces.iter_mut(), particles.velocity.iter(), particles.radius.iter()) {
        let drag = -6.0*PI*viscosity * rad * vel;
        *force += drag;
    }
}

/// Define inelastic collision between particle i and particle j.
/// 
/// This is just worrying about normal forces, no friction
#[inline(always)]
pub fn inelastic_collision(
    i: usize,
    j: usize,
    particles: &ParticleVec,
    forces: &mut [DVec3],
    params: &CollisionParams,
    sim_box_size: &DVec3,
) {
    let stiffness = params.stiffness; 
    let damping = params.damping;     

    //separation of particle centres
    let mut delta = particles.position[i] - particles.position[j];
    check_delta(&mut delta, sim_box_size);
    

    let combined_rad = particles.radius[i] + particles.radius[j];
    let dist = delta.length();

    // Overlap?
    
    if combined_rad > dist && dist > 1e-9 {
        let normal = delta / dist;
        let overlap = combined_rad - dist;

        // Relative Velocity (for Inelastic Damping)
        let rel_vel = particles.velocity[i] - particles.velocity[j];
        let normal_vel = rel_vel.dot(normal);

        // Force Calculation - spring
        let spring_f = stiffness * overlap;

        // Damping only applies when particles are moving towards each other
        let damping_f = if normal_vel < 0.0 {
            -damping * normal_vel
        } else {
            0.0
        };

        let total_f = (spring_f + damping_f).max(0.0);
        let f_vec = normal * total_f;

        // Apply to both (Newton's Third Law)
        forces[i] += f_vec;
        forces[j] -= f_vec;
    }
}

fn check_delta(delta: &mut DVec3, sim_box_size: &DVec3){
    if delta.x > sim_box_size.x * 0.5 { delta.x -= sim_box_size.x; }
    else if delta.x < -sim_box_size.x * 0.5 { delta.x += sim_box_size.x; }

    if delta.y > sim_box_size.y * 0.5 { delta.y -= sim_box_size.y; }
    else if delta.y < -sim_box_size.y * 0.5 { delta.y += sim_box_size.y; }

    if delta.z > sim_box_size.z * 0.5 { delta.z -= sim_box_size.z; }
    else if delta.z < -sim_box_size.z * 0.5 { delta.z += sim_box_size.z; }

}
