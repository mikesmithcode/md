use crate::simulation::SimulationSettings;
use md_core::particle::{Particle, ParticleVec};
use glam::DVec3;

pub trait Forces{
    fn update_forces(&self, particles: &ParticleVec, forces: &mut [DVec3]);
}
