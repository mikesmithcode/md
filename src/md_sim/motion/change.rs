use glam::DVec3;
use itertools::izip;
use three_d::Srgba;

use crate::md_sim::{simulation::SimulationSettings, particle::ParticleVec};


//-------------------------------------------------------------------------------------------------------
// Special functions
//-------------------------------------------------------------------------------------------------------

/// Enforces periodic boundary conditions by wrapping a position into the primary simulation box.
///
/// This function uses a branchless floored-division approach to map any coordinate 
/// $(x, y, z)$ to the range $[0, L)$. If a particle exits the right face of the box, 
/// it "teleports" to the left face, and vice versa.
///
/// # Arguments
///
/// * `pos` - The mutable position vector to be wrapped.
/// * `sim_box_size` - The dimensions of the periodic simulation cell.
///
/// # Physics Context
///
/// The formula used is: $\mathbf{r}_{new} = \mathbf{r} - \mathbf{L} \cdot \lfloor \mathbf{r} / \mathbf{L} \rfloor$.
/// This ensures that the simulation represents an infinite tiling of the 
/// primary cell, maintaining a constant particle density.
pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3) {
    // Branchless wrapping: more efficient than multiple if-statements for large displacements
    *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
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



