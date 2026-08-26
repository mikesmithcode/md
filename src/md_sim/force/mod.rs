// Declare the sub-modules as private but then reexport what's needed to flatten module structue.
mod neighbours;
mod pairwise;
mod single;
mod objects;


// Re-export the traits and key functions for easier access
// This allows you to call forces::Force instead of forces::force::Force
pub use single::{add_weight, add_viscous_drag};
pub use objects::{add_particle_object_collision};
pub use pairwise::{add_particle_particle_collision, add_coulomb};
pub use neighbours::CellGrid;
//pub use bonds::*;



use glam::DVec3;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::{ObjectSpec, SimulationSettings};

#[cfg(test)]
mod tests;

/// Defines the physical interactions, force constraints, and update phases for a simulation.
///
/// The `Forces` trait controls the forces (and torques) applied during the simulation
/// Implementations of this trait define the governing dynamics of a simulation script, 
/// separating computations into pair-wise interactions, single-body forces, object collisions, 
/// or internal molecular forces.
pub trait Forces {
    /// Indicates whether the simulation requires pair-wise force calculations.
    ///
    /// If `false`, the engine skips spatial binning and Verlet list construction, 
    /// significantly improving performance for non-interacting or external-field-only systems.
    fn has_pair_forces(&self) -> bool { true }

    /// Indicates whether the simulation requires single-body force calculations.
    ///
    /// If `false`, the engine will skip the unary `update_single_forces` traversal loop.
    fn has_single_forces(&self) -> bool { true }

    /// Indicates whether the simulation includes interactions between particles and geometric objects.
    ///
    /// Disabled (`false`) by default to avoid unnecessary evaluation overhead.
    fn has_object_forces(&self) -> bool { false }

    /// Indicates whether the simulation requires internal multi-component particle forces.
    ///
    /// Disabled (`false`) by default. Set to `true` if your system uses composite particles 
    /// requiring internal structural force and torque distributions.
    fn has_internal_forces(&self) -> bool { false }

    /// Calculates forces and torques that act on a single particle.
    ///
    /// This method is called once per particle in an $O(N)$ loop. It is designed for 
    /// forces that depend exclusively on a single particle's state, such as gravity, 
    /// viscous drag, external fields, or self-propulsion.
    ///
    /// # Arguments
    ///
    /// * `i` - Index of the particle being updated.
    /// * `force` - Accumulated incoming force vector for particle `i`.
    /// * `torque` - Accumulated incoming torque vector for particle `i`.
    /// * `particles` - Reference to the particle state buffers (positions, velocities, types, etc.).
    /// * `settings` - Global simulation parameters.
    /// * `time` - Current simulation timestamp.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The resulting force and torque adjustments for the particle.
    fn update_single_forces(
        &self, 
        i: usize, 
        force: DVec3, 
        torque: DVec3,
        particles: &ParticleVec, 
        settings: &SimulationSettings,
        time: f64
    ) -> (DVec3, DVec3);

    /// Calculates contact forces (or torques) between individual particles and simulation objects such as Rectangles.
    ///
    /// Objects are passive they can be static or animated but don't respond to particle forces but do apply them to the particle.
    ///
    /// # Arguments
    ///
    /// * `i` - Index of the interacting particle.
    /// * `force` - Accumulated incoming force vector for the particle.
    /// * `torque` - Accumulated incoming torque vector for the particle.
    /// * `particles` - Reference to the particle state buffers.
    /// * `objects` - Reference to the object specification and boundary geometries.
    /// * `settings` - Global simulation parameters.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The resulting force and torque contributions from object interactions.
    fn update_object_forces(
        &self, 
        i: usize, 
        force: DVec3,
        torque: DVec3,
        particles: &ParticleVec, 
        objects: &ObjectSpec,
        settings: &SimulationSettings
    ) -> (DVec3, DVec3);

    /// Calculates interaction forces between two particles within a specified cutoff distance.
    ///
    /// This method is invoked via the `CellGrid` manager for verified neighbour pairs $(i, j)$ 
    /// fetched from the Verlet lists. Implementations should compute potentials like Lennard-Jones, 
    /// electrostatic, or Hertzian contact forces.
    ///
    /// # Arguments
    ///
    /// * `i` - Index of the primary particle.
    /// * `j` - Index of the neighboring particle.
    /// * `force` - Accumulated incoming force vector.
    /// * `torque` - Accumulated incoming torque vector.
    /// * `particles` - Reference to the particle state buffers.
    /// * `settings` - Global simulation parameters.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The force and torque contributions acting on the particle pair.
    fn update_pair_forces(
        &self, 
        i: usize, 
        j: usize, 
        force: DVec3,
        torque: DVec3,
        particles: &ParticleVec, 
        settings: &SimulationSettings
    ) -> (DVec3, DVec3);

    /// Calculates internal forces for molecules composed of multiple particles
    ///
    /// # Arguments
    ///
    /// * `_particles` - Mutable or immutable reference to particle data.
    /// * `_force` - Base force vector buffer.
    /// * `_torque` - Base torque vector buffer.
    /// * `_settings` - Global simulation parameters.
    fn update_internal_forces(
        &self,
        _particles: &ParticleVec, 
        _force: DVec3, 
        _torque: DVec3,
        _settings: &SimulationSettings
    ) {
        // Optional: No internal forces by default.
    }
}
