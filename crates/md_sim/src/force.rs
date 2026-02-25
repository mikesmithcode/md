use crate::simulation::SimulationSettings;
use md_core::particle::{Particle, ParticleVec};
use glam::DVec3;
use itertools::izip;
use std::f64::consts::PI;

pub trait Force{
    fn update_forces(&self, forces: &mut [DVec3], particles: &ParticleVec);

}

///Add the weight
/// 
///assumes g acts in -z direction. If in a fluid your inv_mass particle
///attribute should take into account the relative density.
pub fn add_weight(forces: &mut [DVec3], particles: &ParticleVec) {
    let gravity = -9.81;

    for (force, &inv_mass) in izip!(forces.iter_mut(), particles.inv_mass.iter()) {
        if inv_mass > 0.0 {
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
        let drag = -6.0*PI*viscosity * vel;
        *force += drag;
    }
}
