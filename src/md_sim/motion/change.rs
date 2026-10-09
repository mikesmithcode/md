use glam::DVec3;
use itertools::izip;
use three_d::Srgba;
use std::collections::HashMap;

use crate::md_sim::{SimulationSettings, particle::ParticleVec};
use crate::md_sim::particle::{calculate_molecule_com,MoleculeData};

//-------------------------------------------------------------------------------------------------------
// Special functions
//-------------------------------------------------------------------------------------------------------
/// Wrap molecules
/// 
/// This is used to apply periodic boundary conditions
pub fn periodic_wrap_molecules(
    particles: &mut ParticleVec, 
    molecule_map: &HashMap<usize, MoleculeData>,
    settings: &SimulationSettings
) {
    let box_size = settings.sim_box_size;
    let periodic = settings.periodic;

    for (_, mol) in molecule_map {
        // Calculate the current center of mass
        let (_, com_pos, _) = calculate_molecule_com(&mol.pids, particles);

        // Determine the wrapped COM position for periodic axes
        let mut wrapped_com = com_pos;
        for i in 0..3 {
            if periodic[i] {
                let val = wrapped_com[i] / box_size[i];
                wrapped_com[i] -= box_size[i] * val.floor();
            }
        }

        // Compute the shift vector
        let shift = wrapped_com - com_pos;

        // Apply the shift uniformly to all constituent particles
        if shift.length_squared() > 1e-12 {
            for &idx in &mol.pids {
                particles.position[idx] += shift;
            }
        }
    }
}



/// Enforces boundary conditions (periodic wrapping or elastic reflection) dimension by dimension.
/// 
/// # Arguments
///
/// * `pos` - Mutable reference to the position vector of the particle.
/// * `vel` - Mutable reference to the velocity vector of the particle.
/// * `sim_box_size` - The 3D dimensions of the simulation box.
/// * `periodic` - Boolean array `[bool; 3]` specifying whether each dimension $(x, y, z)$ is periodic (`true`) or bounded (`false`).
/// * `radius` - used so that particles can reflect when they hit edge.
/// 
/// ///
/// # Notes
/// 
/// Each spatial dimension is treated independently:
/// 
///  **Non-Periodic Boundaries (`false`):** Implements perfectly elastic reflection off the box walls:
///    * If the particle crosses the lower boundary ($< 0.0$), position is reflected inward and velocity is inverted ($v_i = -v_i$).
///    * If the particle crosses the upper boundary ($\ge \text{sim\_box\_size}$), position is bounced back relative to the wall and velocity is inverted.
#[inline]
pub fn enforce_boundary(
    pos: &mut DVec3, 
    vel: &mut DVec3, 
    sim_box_size: DVec3, 
    periodic: [bool; 3], 
    radius: f64
) {
    for i in 0..3 {
        if !periodic[i] {
            let min_bound = radius;
            let max_bound = sim_box_size[i] - radius;

            if pos[i] < min_bound {
                pos[i] = min_bound;
                vel[i] = -vel[i]; // Reverse velocity component upon wall collision
            } else if pos[i] > max_bound {
                pos[i] = max_bound;
                vel[i] = -vel[i]; // Reverse velocity component upon wall collision
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
#[inline]
pub fn change_rad(particles: &mut ParticleVec, ptype: usize) {
    for (radius, &p) in izip!(&mut particles.radius, &particles.ptype) {
        if p == ptype {
            *radius *= 1.00001;
        }
    }
}


/// Usually used to move a surface constructed of particles up and down sinusoidally
/// 
/// # Arguments
/// 
/// * `particles` - The mutable particle buffer
/// * `settings` - The SimulationSettings struct which carries the time step equivalent in seconds
/// 
/// # Notes
/// 
/// Need to assign the particles you want to move this way a ptype=1 to distinguish them from other particles
/// The motion is set locally using amplitude and frequency.
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


/// Change colour of particular type of particles.
/// 
/// # Arguments
/// 
/// * `particles` - The mutable particle buffer
/// * `_settings` - unused
/// 
/// # Notes
/// 
/// Currently set to look for particular type of particle above certain height but could look for any condition.
pub fn change_particle_colour(particles: &mut ParticleVec, _settings: &SimulationSettings){
    let threshold: f64 = 0.01;
    
    let new_colour = Srgba::new(0, 255, 0, 255);
    //change colour of particles
    for (pos, col, &ptype) in izip!(&mut particles.position, &mut particles.colour,  &particles.ptype){
        if (ptype == 0) && (pos.z > threshold){
                *col = new_colour; 
            }
    }
}



