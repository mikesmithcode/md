// Declare the sub-modules as private but then reexport what's needed to flatten module structue.
mod neighbours;
mod pairwise;
mod single;
mod objects;
mod contact;


use glam::DVec3;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::{ObjectSpec, Simulation, SimulationSettings};
use std::collections::HashMap;


// Re-export the traits and key functions for easier access
// This allows you to call forces::Force instead of forces::force::Force
pub use contact::{check_particle_contact,check_object_contact, ContactManager, ContactType, Contact};
pub use neighbours::CellGrid;
pub use objects::{check_surface_contact};
pub use pairwise::{normal_linear, normal_hertzian, friction_viscous_damping, friction_cundall_strack, FrictionCundallStrack, FrictionViscous, NormalForce, add_coulomb, CoulombParams};
pub use single::{add_gravity, Gravity, add_viscous_drag, ViscousDrag};
use crate::md_sim::particle::MoleculeData;






#[cfg(test)]
mod tests;

///------------------------------------------------------------------------------
/// Forces Trait
///------------------------------------------------------------------------------
/// Defines the physical interactions, force constraints, and update phases for a simulation.
///
/// The `Forces` trait controls the forces and torques applied during the simulation. 
/// Implementations define the governing dynamics of a simulation script, separating 
/// computations into pair-wise interactions, single-body forces, object collisions, 
/// or internal molecular forces.
pub trait Forces {
    ///------------------------------------------------------------------------------
    /// has_pair_forces
    ///------------------------------------------------------------------------------
    /// Indicates whether the simulation requires pair-wise force calculations.
    ///
    /// If `false`, the engine skips spatial binning and Verlet list construction, 
    /// significantly improving performance for non-interacting or external-field-only systems.
    fn has_pair_forces(&self) -> bool { true }

    ///------------------------------------------------------------------------------
    /// has_single_forces
    ///------------------------------------------------------------------------------
    /// Indicates whether the simulation requires single-body force calculations.
    ///
    /// If `false`, the engine will skip the unary `update_single_forces` traversal loop.
    fn has_single_forces(&self) -> bool { true }

    ///------------------------------------------------------------------------------
    /// has_object_forces
    ///------------------------------------------------------------------------------
    /// Indicates whether the simulation includes interactions between particles and geometric objects.
    ///
    /// Disabled (`false`) by default to avoid unnecessary evaluation overhead.
    fn has_object_forces(&self) -> bool { true }


    ///------------------------------------------------------------------------------
    /// update_single_forces
    ///------------------------------------------------------------------------------
    /// Calculates forces and torques that act on a single particle.
    ///
    /// This method is called once per particle in an $O(N)$ loop. It is designed for 
    /// forces that depend exclusively on a single particle's state, such as gravity, 
    /// viscous drag, external fields, or self-propulsion.
    ///
    /// # Arguments
    ///
    /// * `_i` - Index of the particle being updated.
    /// * `force` - Accumulated incoming force vector for particle `_i`.
    /// * `torque` - Accumulated incoming torque vector for particle `_i`.
    /// * `_particles` - Reference to the particle state buffers (positions, velocities, types, etc.).
    /// * `_settings` - Global simulation parameters.
    /// * `_time` - Current simulation timestamp.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The resulting force and torque adjustments for the particle.
    fn update_single_forces(
        &self, 
        _i: usize, 
        force: DVec3, 
        torque: DVec3,
        _particles: &ParticleVec, 
        _settings: &SimulationSettings,
        _time: f64
    ) -> (DVec3, DVec3) {
        (force, torque)
    }

    ///------------------------------------------------------------------------------
    /// update_object_forces
    ///------------------------------------------------------------------------------
    /// Calculates contact forces (or torques) between individual particles and simulation objects such as Rectangles.
    ///
    /// Objects are passive: they can be static or animated, but do not respond to particle forces 
    /// while still applying forces back to the particle.
    ///
    /// # Arguments
    ///
    /// * `_i` - Index of the interacting particle.
    /// * `force` - Accumulated incoming force vector for the particle.
    /// * `torque` - Accumulated incoming torque vector for the particle.
    /// * `_particles` - Reference to the particle state buffers.
    /// * `_objects` - Reference to the object specification and boundary geometries.
    /// * `_settings` - Global simulation parameters.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The resulting force and torque contributions from object interactions.
    fn update_object_forces(
        &self, 
        _i: usize, 
        force: DVec3,
        torque: DVec3,
        _particles: &ParticleVec, 
        _object_spec: &ObjectSpec,
        _settings: &SimulationSettings
    ) -> (DVec3, DVec3) {
        (force, torque)
    }

    ///------------------------------------------------------------------------------
    /// update_pair_forces
    ///------------------------------------------------------------------------------
    /// Calculates interaction forces between two particles within a specified cutoff distance.
    ///
    /// This method is invoked via the `CellGrid` manager for verified neighbor pairs $(i, j)$ 
    /// fetched from the Verlet lists. Implementations should compute potentials like Lennard-Jones, 
    /// electrostatic, or Hertzian contact forces.
    ///
    /// # Arguments
    ///
    /// * `_i` - Index of the primary particle.
    /// * `_j` - Index of the neighboring particle.
    /// * `force` - Accumulated incoming force vector.
    /// * `torque` - Accumulated incoming torque vector.
    /// * `_particles` - Reference to the particle state buffers.
    /// * `_settings` - Global simulation parameters.
    ///
    /// # Returns
    ///
    /// * `(DVec3, DVec3)` - The force and torque contributions acting on the particle pair.
    fn update_pair_forces(
        &self, 
        _i: usize, 
        _j: usize, 
        force: DVec3,
        torque: DVec3,
        _particles: &ParticleVec, 
        _settings: &SimulationSettings
    ) -> (DVec3, DVec3) {
        (force, torque)
    }



    ///------------------------------------------------------------------------------
    /// handle_contact_separation
    ///------------------------------------------------------------------------------
    /// 1. Performs cleanup tasks for contact managers or state caches at the end of an integration step.
    /// 2. Can also be used to add charges to particles as they are separated
    /// /// ```rust
    /// fn handle_contact_separation(&mut self, particles: &mut ParticleVec) {
    ///     self.contact_manager.prune_separated_contacts(|contact_type, state| {
    ///         // `contact_type` tells you which two particles were in contact.
    ///         // `state` gives you the last known state of that contact.
    ///         // Example: add a unit of charge to each particle when they separate.
    ///         let (i, j) = contact_type.pair_indices();
    ///         particles[i].charge += 1.0;
    ///         particles[j].charge += 1.0;
    ///         // You can also log, emit events, etc.
    ///         println!("Contact between {} and {} ended; charge transferred.", i, j);
    ///      });
    /// }
    /// ```
    fn handle_contact_separation(&mut self,
        _particles: &mut ParticleVec, 
        _molecule_map: &HashMap<usize, MoleculeData>,
        _settings: &SimulationSettings
        ) {
            
        }
}
