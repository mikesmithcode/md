use glam::DVec3;
use itertools::izip;
use three_d::Srgba;

use crate::md_sim::{SimulationSettings, particle::ParticleVec};


//-------------------------------------------------------------------------------------------------------
// Special functions
//-------------------------------------------------------------------------------------------------------

/// Enforces boundary conditions
/// 
/// periodic specifies either true (periodic) or false (not) for each dimension.
/// If a BC is periodic it wraps the position in the primary simulation box. (The forces are also wrapped
/// in the neighbours). If a BC is not periodic the appropriate faces of the simulation box are perfectly elastic
/// reflecting particles and forces do not wrap.
///
/// # Arguments
///
/// * `pos` - The mutable position vector to be wrapped.
/// * `vel` - The mutable velocity vector to be reflected.
/// * `sim_box_size` - The dimensions of the periodic simulation cell.
/// * `periodic` - true = periodic and false = non-periodic. Each dimension is treated separately
#[inline(always)]
pub fn enforce_boundary(pos: &mut DVec3, vel: &mut DVec3, sim_box_size: DVec3, periodic: [bool; 3]) {
    for i in 0..3 {
        if periodic[i] {
            let val = pos[i] / sim_box_size[i];
            pos[i] -= sim_box_size[i] * val.floor();
        } else {
            // Precise Elastic Reflection
            if pos[i] < 0.0 {
                pos[i] = -pos[i]; // Reflect from 0
                vel[i] = -vel[i]; // Reverse velocity
            } else if pos[i] >= sim_box_size[i] {
                // Reflect from box_size: 
                // The distance past the wall is (pos[i] - sim_box_size[i])
                // We subtract that distance from the wall to bounce back
                pos[i] = 2.0 * sim_box_size[i] - pos[i];
                vel[i] = -vel[i];
            }
        }
    }
}

/// Incrementally increases the radius of particles belonging to a specific type.
///
/// This is typically used in "compression-by-growth" protocols to reach a 
/// jammed state or to simulate swelling materials.
///
/// # Arguments
///
/// * `particles` - The mutable particle buffer.
/// * `ptype` - The specific particle category ID that should undergo growth.
///
/// # Notes
///
/// * **Mass Consistency:** Note that this only modifies the `radius` field. 
///   If your simulation physics depends on `mass`, you may need to 
///   recalculate it after calling this function to maintain a constant density.
/// * **Growth Rate:** The current multiplier is $1.00001$ ($0.001\%$) per call.
pub fn change_rad(particles: &mut ParticleVec, ptype: usize) {
    for (radius, &p) in izip!(&mut particles.radius, &particles.ptype) {
        if p == ptype {
            *radius *= 1.00001;
        }
    }
}

pub fn move_sinwave(particles: &mut ParticleVec, settings: &SimulationSettings, time: f64){
    let amplitude: f64 = 0.1;
    let frequency: f64= 250.0;

    //move surface particles up and down
    for (pos, &ptype) in izip!(&mut particles.position, &particles.ptype){
        if ptype == 1{
            let velocity_z = amplitude*(2.0*std::f64::consts::PI*frequency*time).cos();
            pos.z += velocity_z * settings.dt;
        }
    }

}

pub fn change_colour(particles: &mut ParticleVec, _settings: &SimulationSettings){
    let threshold: f64 = 0.01;
    
    let new_colour = Srgba::new(0, 255, 0, 255);
    //change colour of particles
    for (pos, col, &ptype) in izip!(&mut particles.position, &mut particles.color,  &particles.ptype){
        if (ptype == 0) && (pos.z > threshold){
                *col = new_colour; 
            }
    }
}



