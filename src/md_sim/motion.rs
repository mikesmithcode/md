use crate::{SimulationSettings, md_sim::particle::ParticleVec};
use glam::DVec3;
use itertools::izip;

/// Motion is a trait which must be implemented for a SimUpdate Struct.
/// 
/// You need to define an update_motion function that will call stand alone
/// functions in motion.rs. All integration, boundary conditions, collisions etc are performed in this function.
pub trait Motion{
    fn update_motion(&self, forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings);
    fn correct_motion(&self, forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings);
}


/// Perform verler integration
/// 
/// needs to be applied in update and correct motion.
pub fn integrate_verler(forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings){
    let dt = settings.dt;
    let sim_box_size = settings.sim_box_size;

    // We zip the columns of the ParticleVec along with the external forces slice
    for (pos, vel, inv_mass, force) in izip!(
        &mut particles.position,
        &mut particles.velocity,
        &particles.inv_mass,
        forces
    ) {
        let acceleration = *force * (*inv_mass);
        *vel += acceleration * dt;
        *pos += *vel * dt;
        check_periodic(pos, sim_box_size);
    }
}

/// Perform Euler integration
/// 
/// velocities are adjusted due to forces and then positions due to velocities. The function checks
/// whether particle has left simulation box and applies periodic boundary conditions.
pub fn integrate_euler(forces: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings){
    
    let dt = settings.dt;
    let sim_box_size = settings.sim_box_size;

    // We zip the columns of the ParticleVec along with the external forces slice
    for (pos, vel, inv_mass, force) in izip!(
        &mut particles.position,
        &mut particles.velocity,
        &particles.inv_mass,
        forces
    ) {
        let acceleration = *force * (*inv_mass);
        *vel += acceleration * dt;
        *pos += *vel * dt;
        check_periodic(pos, sim_box_size);
    }
}




pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3){
    *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
}
