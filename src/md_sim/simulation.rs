//! The simulation is run by calling methods on the Simulation struct.
//! 
//! The `Simulation` struct is the centerpiece of the simulation engine. 
//! Create an instance with [`Simulation::new()`] and update it each timestep with [`Simulation::update()`].
//! 
//! To use it, one must implement and pass a custom update struct adhering to the [`Motion`] and [`Forces`] traits,
//! along with a [`ParticleVec`] initialized with starting positions.
//! 
//! ```rust
//! pub struct SimUpdate;
//!
//! impl Forces for SimUpdate {
//!     // Forces which apply to every particle individually
//!     fn update_single_forces(&self, i: usize, mut force: DVec3, torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64) -> (DVec3, DVec3) {   
//!         (force, torque)
//!     }
//!     // Forces that operate between pairs of particles
//!     fn update_pair_forces(&self, i: usize, j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec, settings: &SimulationSettings) -> (DVec3, DVec3) {
//!         (force, torque)
//!     }
//! }
//!
//! impl Motion for SimUpdate {
//!     fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>, _time: f64) {
//!         integrate_rigid_bodies(forces, torques, particles, molecule_map, settings);
//!     }
//!     fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec, settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>) {
//!         integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings)
//!     }
//! }
//! ```

use glam::DVec3;
use std::collections::HashMap;
use itertools::izip;

use crate::md_sim::ObjectSpec;
use crate::md_sim::particle::MoleculeData;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::force::CellGrid;
use crate::md_sim::Forces;
use crate::md_sim::Motion;
use crate::md_sim::SimulationSettings;


/// The main simulation engine orchestrating particle states, forces, boundary grids, and time integration steps.
/// 
/// Requires a user-defined struct implementing both the [`Forces`] and [`Motion`] traits to define simulation behavior.
#[derive(Debug)]
pub struct Simulation<S> 
    where 
        S: Forces + Motion,
{
    pub particles: ParticleVec,
    pub objects: Option<Vec<ObjectSpec>>,
    pub forces: Vec<DVec3>,
    pub torques: Vec<DVec3>,
    pub sim_update: S,
    pub settings: SimulationSettings,
    pub current_step: usize,
    pub cell_grid: CellGrid,
    pub time: f64,
    pub molecule_map: HashMap<usize, MoleculeData>,
}

impl<S> Simulation<S> 
    where 
        S: Forces + Motion + Sync,
{
    /// Creates and initializes a new simulation instance, building molecule groupings and setting up the spatial cell grid.
    pub fn new(mut particles: ParticleVec, objects: Option<Vec<ObjectSpec>>, sim_update: S, settings: SimulationSettings, time: f64) -> Self {
        let n = particles.len();
        let molecule_map = build_molecule_map(&particles);
        let mut cell_grid = CellGrid::new(n, &settings);
        cell_grid.init(&mut particles, &settings);

        Self {
            particles,
            objects,
            forces: vec![DVec3::ZERO; n],
            torques: vec![DVec3::ZERO; n],
            sim_update,
            settings: settings.clone(),
            current_step: settings.start,
            cell_grid,
            time,
            molecule_map,
        }
    }

    /// Advances the simulation by a single time step using a velocity-Verlet predictor-corrector scheme.
    /// 
    /// 1. Predicts new particle positions and velocities via [`Motion::update_motion`].
    /// 2. Updates positions of any active simulation objects.
    /// 3. Resets and accumulates single-particle, object-interaction, and pairwise forces.
    /// 4. Corrects velocities via [`Motion::correct_motion`] based on the newly computed forces.
    /// 5. Increments the simulation clock by the time step size (`dt`).
    pub fn update(&mut self) {
        //----------------------------------------------------------------------------
        // Initial position and velocity updates
        //----------------------------------------------------------------------------
        
        // Predict the new positions, velocities, etc.
        self.sim_update.update_motion(&self.forces, &self.torques, &mut self.particles, &self.settings, &self.molecule_map, self.time);

        // Move any objects
        if let Some(scene_objects) = self.objects.as_deref_mut() {
            for object in scene_objects {
                self.sim_update.update_objects(object, &self.settings, self.time);
            }
        }

        //----------------------------------------------------------------------------
        // Calculate all the forces
        //----------------------------------------------------------------------------
        self.reset_forces();

        if self.sim_update.has_single_forces() {
            let (mut force, mut torque);
            for i in 0..self.particles.len() {
                (force, torque) = self.sim_update.update_single_forces(i, DVec3::ZERO, DVec3::ZERO, &self.particles, &self.settings, self.time);
                self.forces[i] += force;
                self.torques[i] += torque;
            }
        }

        if self.sim_update.has_object_forces() {
            if let Some(objects) = self.objects.as_deref() {
                for i in 0..self.particles.len() {
                    let mut total_force = DVec3::ZERO;
                    let mut total_torque = DVec3::ZERO;

                    // Loop through every object in the slice
                    for obj in objects {
                        let (force, torque) = self.sim_update.update_object_forces(
                            i, 
                            DVec3::ZERO, 
                            DVec3::ZERO, 
                            &self.particles, 
                            obj, 
                            &self.settings
                        );
                        total_force += force;
                        total_torque += torque;
                    }

                    self.forces[i] += total_force;
                    self.torques[i] += total_torque;
                }
            }
        }


        // Grid checks nearby particles and calculates all pair forces between particles i and j.
        if self.sim_update.has_pair_forces() {
            // Check if grid and verlet lists need recalculating
            self.cell_grid.check_and_rebuild_neighbours(&mut self.particles, &self.settings);
            // Apply pairwise forces
            self.cell_grid.apply_pair_forces(
                &mut self.forces, 
                &mut self.torques,
                &self.particles, 
                &self.sim_update, 
                &self.settings
            );
        }

        //----------------------------------------------------------------------------
        // Final velocity correction updates
        //----------------------------------------------------------------------------
        

        // Perform correction to the motion based on the updated forces
        self.sim_update.correct_motion(&self.forces, &self.torques, &mut self.particles, &self.settings, &self.molecule_map);
        
        // Update simulation time
        self.time += self.settings.dt;
    }

    /// Returns an immutable reference to the particle collection.
    pub fn get_particles(&self) -> &ParticleVec {
        &self.particles
    }

    /// Returns a mutable reference to the particle collection.
    pub fn get_mut_particles(&mut self) -> &mut ParticleVec {
        &mut self.particles
    }

    /// Returns an immutable reference to the simulation objects, if present.
    pub fn get_objects(&self) -> Option<&[ObjectSpec]> {
        self.objects.as_deref()
    }

    // Resets the force and torque vectors to zero, resizing them if particle counts have changed.
    fn reset_forces(&mut self) {
        if self.forces.len() != self.particles.len() {
            self.forces.resize(self.particles.len(), DVec3::ZERO);
            self.torques.resize(self.particles.len(), DVec3::ZERO);
        } else {
            self.forces.fill(DVec3::ZERO);
            self.torques.fill(DVec3::ZERO);
        }
    }
}

/// Groups particle identifiers by their molecular IDs and constructs inertial mapping data for each molecule.
fn build_molecule_map(particles: &ParticleVec) -> HashMap<usize, MoleculeData> {
    // Group indices by mol_id
    let mut temp_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (id, &mol_id) in izip!(&particles.id, &particles.molecule_id) {
        temp_map.entry(mol_id).or_default().push(*id);
    }

    // Convert to MoleculeData - store inertia
    let mut molecule_map = HashMap::new();
    for (mol_id, pids) in temp_map {
        let mol_data = MoleculeData::new(pids, particles);
        molecule_map.insert(mol_id, mol_data);
    }

    molecule_map
}
