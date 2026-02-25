use md_core::particle::ParticleVec;
use glam::DVec3;
use itertools::izip;

pub trait Move{
    fn update_motion(&self, forces: &mut [DVec3], particles: &ParticleVec, dt: f64);
}


pub fn integrate(forces: &mut [DVec3], particles: &mut ParticleVec, dt: f64){
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
    }
}

pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3){
    *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
}
