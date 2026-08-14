use std::collections::HashMap;
use glam::DVec3;

mod change;
mod integration;

pub use change::{enforce_boundary, change_rad, move_sinwave, change_colour};
pub use integration::{integrate_singleparticle_update,integrate_singleparticle_correct, integrate_rigid_bodies, integrate_rigid_bodies_correct, update_abps};

pub use crate::md_sim::{SimulationSettings, ParticleVec, ObjectSpec};
pub use super::particle::MoleculeData;

#[cfg(test)]
mod tests;

/// Defines the integration scheme and kinematic updates for the simulation.
///
/// The `Motion` trait is responsible for advancing the simulation in time. It 
/// handles the numerical integration of Newton's laws of motion, as well as 
/// the application of boundary conditions (e.g., periodic wrapping or wall reflections).
pub trait Motion {
    /// Advances particle states at the start of a simulation step (Prediction).
    ///
    /// This method is typically called before force accumulation. In a standard 
    /// Velocity Verlet scheme, this is used to update positions based on current 
    /// velocities and to perform a "half-step" update to velocities using 
    /// **previous** force data.
    ///
    /// # Arguments
    ///
    /// * `forces` - The force buffer calculated during the **previous** timestep.
    /// * `particles` - The mutable particle data to be updated.
    /// * `settings` - Global simulation parameters, including the timestep ($\Delta t$).
    ///
    /// # Implementation Note
    /// Standalone integration functions (like `verlet_predict`) 
    /// should be called within this method to maintain a modular design.
    fn update_motion(
        &self, 
        _forces: &[DVec3], 
        _torques: &[DVec3],
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings,
        _molecule_map: &HashMap<usize, MoleculeData>,
        _time: f64
    );

    /// Finalises particle states at the end of a simulation step (Correction).
    ///
    /// This is an optional hook called after the **current** forces have been 
    /// accumulated. It is primarily used in multi-step integrators to correct 
    /// velocities using the newly calculated force data.
    ///
    /// # Default Implementation
    /// The default implementation is empty, making this step optional.
    ///
    /// # Arguments
    ///
    /// * `forces` - The force buffer calculated during the **current** timestep.
    /// * `particles` - The mutable particle data to be corrected.
    /// * `settings` - Global simulation parameters.
    fn correct_motion(
        &self, 
        _forces: &[DVec3], 
        _torques: &[DVec3],
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings,
        _molecule_map: &HashMap<usize, MoleculeData>
    ) {
        // Optional: No correction by default
    }


    /// Can be used to update any property of the ObjectSpec. 
    /// If Option<Vec<ObjectSpec>> = None this is bypassed otherwise Vec<ObjectSpec> extracted
    /// and modified in place.
    fn update_objects(&self, _objects: &mut [ObjectSpec], _settings: &SimulationSettings, _time: f64){
        //Optional no movement by default. It is assumed all objects are passive moving according to prescribed rules
        //for obj in objects {
        //match obj {
        //    ObjectSpec::Rect(rect) => {
        //        // Example: apply a velocity or prescribed motion
        //        rect.transform(translation_delta, Some(rotation_delta));
        //    }
        //    ObjectSpec::Tri(tri) => {
        //        tri.transform(translation_delta, Some(rotation_delta));
        //    }   
        // }
    //}
    }
}

