///--------------------------------------------------------------------------------------------
/// SINGLE PARTICLE FORCES
/// -------------------------------------------------------------------------------------------

use glam::DVec3;
use rand_distr::{Normal, Distribution};
use std::f64::consts::PI;

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::md_sim::utils::models::SimulationModel;

/// Calculates and adds the gravitational weight to a specific particle.
///
/// This function assumes a constant gravitational acceleration $g \approx 9.81 \, \text{m/s}^2$ 
/// acting in the negative $z$ direction.
///
/// # Arguments
///
/// * `i` - The index of the target particle within the force and particle buffers.
/// * `forces` - A mutable slice of force vectors to which the weight will be added.
/// * `particles` - A reference to the particle data structure containing inverse masses.
///
/// # Notes
///
/// * **Buoyancy:** If simulating a fluid environment, the `mass` attribute of the 
///   particle should be adjusted to reflect the effective weight (relative density).
/// * **Infinite Mass:** Particles with an `mass` of **0.0**  are skipped to avoid division by zero.
///
/// # Panics
///
/// This function will panic if the index `i` is out of bounds for either `forces` 
/// or `particles.mass`.
pub fn add_weight(i: usize, forces: &mut [DVec3], particles: &ParticleVec) {
    let gravity = -9.81;
    let mass = particles.mass[i];

    let weight = gravity * mass;
    forces[i].z += weight;
}


/// Calculates and adds the viscous drag force (Stokes' Law) to a specific particle.
///
/// This function models the drag force exerted on a spherical particle moving through 
/// a viscous fluid at low Reynolds numbers, where the force is proportional to the 
/// particle's velocity and radius.
///
/// # Mathematical Formula
///
/// The drag force is calculated as:
/// $$F_{drag} = -6\pi \eta r v$$
/// where $\eta$ is the dynamic viscosity, $r$ is the particle radius, and $v$ is the velocity.
///
/// # Arguments
///
/// * `i` - The index of the target particle.
/// * `forces` - A mutable slice of force vectors to which the drag will be added.
/// * `particles` - A reference to the particle data structure containing velocity and radius.
/// * `viscosity` - The dynamic viscosity ($\eta$) of the surrounding fluid.
///
/// # Panics
///
/// This function will panic if the index `i` is out of bounds for `forces`, 
/// `particles.velocity`, or `particles.radius`.
pub fn add_viscous_drag(i: usize, forces: &mut [DVec3], particles: &ParticleVec, viscosity: f64) {
    let vel = particles.velocity[i];
    let rad = particles.radius[i];
    
    // Stokes' Law: F = -6 * pi * eta * r * v
    let drag = -6.0 * PI * viscosity * rad * vel;
    
    forces[i] += drag;
}

/// An active force for use with ABPs
/// 
/// For each particle i we generate some random numbers. We then calculate the noise scale.
/// The variance of the random displacement in time dt is 2*Dt*dt but we will multiply this by 
/// dt when we calculate the displacement in motion part. Friction F is gamma * v. The noise must be (2*gamma**2 * Dt/dt)**0.5
pub fn add_active_force(i: usize, forces: &mut [DVec3], particles: &ParticleVec, settings: &SimulationSettings){
    let mut rng = rand::thread_rng();
    let normal = Normal::new(0.0, 1.0).unwrap();

    if let SimulationModel::Active(params) = &settings.model {
        // initial direction       
        let dir_vector = particles.orientation[i] * particles.rel_pos[i];

        // F_active = gamma * v0 * dir_vector in direction of particle orientation
        let f_active = dir_vector*(params.gamma * params.v0);

        // Translational Noise "Force"
        // This represents the random kicks from the surrounding fluid
        let noise_scale = 0.0;//params.gamma * (2.0 * params.Dt / settings.dt).sqrt();
        
        let f_noise = glam::DVec3::new(
            normal.sample(&mut rng) * noise_scale,
            0.0, 
            normal.sample(&mut rng) * noise_scale,
        );

        // Add force
        forces[i] += f_active + f_noise;
    }
    
}

