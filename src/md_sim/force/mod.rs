// Declare the sub-modules as private but then reexport what's needed to flatten module structue.
mod bonds;
mod neighbours;
mod pairwise;
mod single;


// Re-export the traits and key functions for easier access
// This allows you to call forces::Force instead of forces::force::Force
pub use single::{add_weight, add_viscous_drag, add_active_force};
pub use pairwise::{add_granular_collision, add_weeks_chandler_andersen, add_coulomb};
pub use neighbours::CellGrid;
pub use bonds::*;



use glam::DVec3;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;

#[cfg(test)]
mod tests;


/// Defines the physical interactions and force constraints for a simulation.
///
/// The `Forces` trait is the core of the physics engine's dynamics. It separates 
/// force calculations into unary (single-body), binary (pair-wise), and 
/// post-processing (constraints) phases.
pub trait Forces {
    /// Indicates if the simulation requires pair-wise force calculations.
    ///
    /// If `false`, the engine will skip spatial binning and the 
    /// `update_pair_forces` calls, significantly improving performance.
    fn has_pair_forces(&self) -> bool { true }

    /// Indicates if the simulation requires single-body force calculations.
    ///
    /// If `false`, the engine will skip the `update_single_forces` loop.
    fn has_single_forces(&self) -> bool { true }

    /// Set this to true if your particle is composite and you need to apply forces and torques to whole.
    fn has_internal_forces(&self) -> bool {false}

    /// Calculates unary forces acting on a specific particle.
    ///
    /// This method is called once per particle in an $O(N)$ loop. It is the 
    /// ideal location for forces that depend only on an individual particle's 
    /// state, such as gravity, viscous drag, or self-propulsion.
    ///
    /// # Arguments
    /// * `i` - Index of the particle being updated.
    /// * `particles` - Reference to the particle data (positions, velocities, etc.).
    /// * `settings` - Global simulation parameters.
    /// 
    /// Returns force and torque for a single particle
    fn update_single_forces(
        &self, 
        i: usize, 
        force: DVec3, 
        torque: DVec3,
        particles: &ParticleVec, 
        settings: &SimulationSettings,
        time: f64
    )->(DVec3, DVec3);

    /// Calculates binary forces between two particles within the cutoff distance.
    ///
    /// This method is called by the `CellGrid` manager for pairs $(i, j)$ found 
    /// within neighbouring cells. Users should calculate the interaction (e.g., 
    /// Lennard-Jones or Hertzian contact) and update the force buffer for both 
    /// particles if Newton's Third Law applies.
    ///
    /// # Arguments
    /// * `i`, `j` - Indices of the interacting particles.
    /// 
    /// # Returns
    /// * (force,torque)` - The force and torque acting on a single particle
    fn update_pair_forces(
        &self, 
        i: usize, 
        j: usize, 
        force: DVec3,
        torque: DVec3,
        particles: &ParticleVec, 
        settings: &SimulationSettings
    )->(DVec3, DVec3);

    fn update_internal_forces(
        &self,
        _particles: &ParticleVec, 
        _force: DVec3, 
        _torque: DVec3,
        _settings: &SimulationSettings
    ){
        // Optional: No Internal Forces by Default
    }
}
