use crate::simulation::{SimulationSettings, NoExtraParams, FluidParams, Fluid};
use md_core::particle::Particle;
use glam::DVec3;

pub trait Forces{
    fn update_forces(&self, particles: &[Particle], forces: &mut [DVec3]);
}

impl Forces for NoExtraParams {
    fn update_forces(&self, particles: &[Particle], forces: &mut [DVec3]) {
        calc_gravity(particles, forces);
    }
}

impl Forces for FluidParams {
    fn update_forces(&self, particles: &[Particle], forces: &mut [DVec3]) {
        calc_gravity(particles, forces);
        calc_drag(particles, forces, self);
    }
}

///Calculates the force due to gravity.
/// 
/// inv_mass is (the difference in density between particle and fluid * g)^-1
pub fn calc_gravity(particles: &[Particle], forces: &mut[DVec3]){
    let gravity_constant = -9.81;

    for (particle, force) in particles.iter().zip(forces.iter_mut()) {
        let apparent_weight = gravity_constant / particle.inv_mass;
        force.z += apparent_weight;
    }
}

///Stokes like drag
pub fn calc_drag(particles: &[Particle], forces: &mut[DVec3], settings: &FluidParams){   
    let viscosity = settings.viscosity();
    let drag_coeff = -6.0 * std::f64::consts::PI * viscosity;

    for (particle, force) in particles.iter().zip(forces.iter_mut()) {
        *force += particle.velocity * drag_coeff * particle.radius;
    }
}
