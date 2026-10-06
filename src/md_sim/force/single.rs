///--------------------------------------------------------------------------------------------
/// SINGLE PARTICLE FORCES
/// -------------------------------------------------------------------------------------------

use glam::DVec3;
use std::f64::consts::PI;

use crate::md_sim::particle::ParticleVec;


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
#[inline]
pub fn add_gravity(i: usize, mut force: DVec3, particles: &ParticleVec, gravity: &Gravity)-> DVec3 {
    let g = gravity.g * gravity.up;
    let mass = particles.mass[i];
    let weight =  mass * g;
    force += weight;
    force
}

pub struct Gravity{
    pub g: f64,
    pub up: DVec3,
}

impl Default for Gravity{
    fn default() -> Self {
        Self { 
            g: -9.81,
            up: DVec3::Z,
        }
    }
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
#[inline]
pub fn add_viscous_drag(i: usize, particles: &ParticleVec, mut force: DVec3, viscous_drag: &ViscousDrag) -> DVec3{
    let vel = particles.velocity[i];
    let rad = particles.radius[i];
    
    // Stokes' Law: F = -6 * pi * eta * r * v
    let drag = -6.0 * PI * viscous_drag.viscosity * rad * vel;
    
    force += drag;
    force
}

pub struct ViscousDrag{
    pub viscosity: f64,
}

impl Default for ViscousDrag{
    fn default()-> Self{        
        Self { viscosity: 1.0 }
    }
}





