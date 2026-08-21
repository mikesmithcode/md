use std::collections::HashMap;
use glam::DVec3;

mod change;
mod integration;

pub use change::{enforce_boundary, change_rad, move_sinwave, change_particle_colour};
pub use integration::{integrate_singleparticle_update,integrate_singleparticle_correct, integrate_rigid_bodies, integrate_rigid_bodies_correct};

pub use crate::md_sim::{SimulationSettings, ParticleVec, ObjectSpec};
pub use super::particle::MoleculeData;

#[cfg(test)]
mod tests;

/// Defines the integration scheme, kinematic updates, and boundary conditions for the simulation.
///
/// The `Motion` trait is responsible for advancing physical states in time. It 
/// handles the numerical integration of Newton's laws of motion (via Velocity Verlet) 
/// for both standard point particles and complex rigid-body molecules, while also 
/// managing prescribed motion for environmental objects.
pub trait Motion {
    /// Advances particle and molecular states at the start of a simulation step (Prediction).
    ///
    /// This method is called before force accumulation. In a Velocity Verlet scheme, 
    /// it updates positions based on current velocities and performs a "half-step" 
    /// velocity update using forces from the **previous** timestep.
    ///
    /// # Arguments
    ///
    /// * `forces` - Slice of force vectors calculated during the previous timestep. One per particle.
    /// * `torques` - Slice of torque vectors calculated during the previous timestep. One per particle.
    /// * `particles` - Mutable reference to the particle buffers containing positions, velocities, orientations, etc.
    /// * `settings` - Global simulation parameters, including the timestep ($\Delta t$) and box dimensions.
    /// * `molecule_map` - Mapping of molecule IDs to their constituent particle IDs and inertial properties.
    /// * `time` - Current simulation time.
    ///
    /// # Notes
    /// 
    /// Implementations should delegate to standalone integration functions (such as 
    /// `integrate_singleparticle_update` or `integrate_rigid_bodies`) to maintain a clean, modular design.
    fn update_motion(
        &self, 
        _forces: &[DVec3], 
        _torques: &[DVec3],
        _particles: &mut ParticleVec, 
        _settings: &SimulationSettings,
        _molecule_map: &HashMap<usize, MoleculeData>,
        _time: f64
    );

    /// Finalizes particle and molecular states at the end of a simulation step (Correction).
    ///
    /// This hook is called after the **current** forces and torques have been accumulated. 
    /// It completes the Velocity Verlet cycle by correcting velocities using the newly 
    /// evaluated force data.
    ///
    /// # Arguments
    ///
    /// * `forces` - Slice of force vectors calculated at the **new** positions. One per particle.
    /// * `torques` - Slice of torque vectors calculated at the **new** positions. One per particle.
    /// * `particles` - Mutable reference to the particle buffers to be corrected.
    /// * `settings` - Global simulation parameters, including the timestep ($\Delta t$).
    /// * `molecule_map` - Mapping of molecule IDs to their constituent particle IDs and inertial properties.
    ///
    /// # Notes
    ///
    /// * **Default Behavior:** The default implementation is empty, making this step optional for 
    ///   simple integrators.
    /// * **Delegation:** Implementations should call correction functions like `integrate_singleparticle_correct` 
    ///   or `integrate_rigid_bodies_correct`.
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

    /// Updates the kinematic properties or prescribed motion of environmental boundary objects.
    ///
    /// # Arguments
    ///
    /// * `object` - Mutable reference to a single `ObjectSpec` (e.g., a rectangle or triangle boundary).
    /// * `settings` - Global simulation parameters.
    /// * `time` - Current simulation time.
    ///
    /// # Notes
    ///
    /// * **Default Behavior:** By default, objects are assumed to be passive or static (no movement).
    /// * **Prescribed Motion:** Override this method to apply time-dependent trajectories (e.g., oscillating walls 
    ///   via sine waves) by matching on the object variants and calling `.transform(...)`.
    fn update_objects(&self, _object: &mut ObjectSpec, _settings: &SimulationSettings, _time: f64) {
        //Optional no movement by default. It is assumed all objects are passive moving according to prescribed rules
        //match object {
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

