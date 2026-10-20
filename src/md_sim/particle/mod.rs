mod analysis;
mod geometry;
mod particle;
mod objects;


#[cfg(test)]
mod tests;

pub use analysis::{calculate_kinetic_energy, calculate_total_angular_momentum};
pub use geometry::{MoleculeData, calculate_molecule_com, calculate_molecule_inertia, find_closest_charge};
pub use particle::{Particle, ParticleVec};
pub use objects::{ObjectSpec, RectSpec, TriSpec, BoxSpec, LineSpec, SurfaceKinematics};
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Charging{
    pub dq_pp:f64,
    pub dq_ps:f64,
    pub q_max: f64,
}