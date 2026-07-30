//! The simulation is run by calling methods on the Simulation struct
//! 
//! The Simulation struct is the centrepiece of the simulation. Define one with the ::new() and then
//! update it each timestep with update(). You can get mutable or immmutable references to the particles. The immutable
//! ones can be fed to the visualization.
//! 

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

/// The main simulation engine
/// 
/// Requires a user defined struct which implements two Traits Forces and Motion.
/// The method stubs need to be filled in by user to define what happens in the simulation.
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
        /// Create a new simulation
        pub fn new(mut particles: ParticleVec, objects: Option<Vec<ObjectSpec>>, sim_update: S, settings: SimulationSettings, time: f64) -> Self {
            let n = particles.len();
            let molecule_map = build_molecule_map(&particles);
            let mut cell_grid=CellGrid::new( n, &settings);
            cell_grid.init(&mut particles, &settings);

            Self {
                particles,
                objects,
                forces : vec![DVec3::ZERO; n],
                torques : vec![DVec3::ZERO; n],
                sim_update,
                settings: settings.clone(),
                current_step: settings.start,
                cell_grid,
                time,
                molecule_map
            }
        }

        /// Update the simulation
        /// 
        /// The positions and velocities are updated in 2 steps
        /// First we predict the motion based on current values
        /// Then we calculate the forces
        /// If there are any particles that shouldn't respond to the forces (walls, prescribed motion) we call a method
        /// which zeros those elements of the force vector.
        /// Then we correct our prediction in light of the new forces.
        /// Only pairs of particles within the cutoff distance are 
        /// calculated for the pair forces.
        pub fn update(&mut self){

            // Predict the new positions, velocities etc
            self.sim_update.update_motion(&self.forces, &self.torques, &mut self.particles, &self.settings,&self.molecule_map, self.time);


            //----------------------------------------------------------------------------
            // Calculate all the forces
            //----------------------------------------------------------------------------
            if self.sim_update.has_single_forces() || self.sim_update.has_pair_forces(){
                //Clear the force buffer and check same length as particles
                self.reset_forces();
            }

            if self.sim_update.has_single_forces(){
                // Single forces apply to individual particles
                let (mut force, mut torque);
                for i in 0..self.particles.len(){
                    (force, torque) = self.sim_update.update_single_forces(i, DVec3::ZERO, DVec3::ZERO, &self.particles, &self.settings, self.time);
                    self.forces[i] += force;
                    self.torques[i] += torque;
                }
            }

            // Grid means you only check particles nearby. Then it calculates all pair forces 
            // between particles i and j.
            if self.sim_update.has_pair_forces(){
                //Check if grid and verlet lists need recalculating
                self.cell_grid.check_and_rebuild_neighbours(&mut self.particles, &self.settings);
                //apply pairwise forces
                self.cell_grid.apply_pair_forces(
                    &mut self.forces, 
                    &mut self.torques,
                    &self.particles, 
                    &self.sim_update, 
                    &self.settings
                    );
            }

            //if self.sim_update.has_internal_forces(){
            //    for i in 0..self.particles.len() {
            //        todo!();
            //        
            //    }
            //}

            // Perform correction to the motion based on the updated forces
            self.sim_update.correct_motion(&self.forces, &self.torques, &mut self.particles, &self.settings, &self.molecule_map);

            // Update simulation time
            self.time += self.settings.dt;
            
        }

        pub fn get_particles(&self)-> &ParticleVec{
            &self.particles
        }

        pub fn get_mut_particles(&mut self)-> &mut ParticleVec{
            &mut self.particles
        }

        pub fn get_objects(&self) -> Option<&[ObjectSpec]>{
            self.objects.as_deref()
        }

        

        /// Reset the force vec to Zeros
        /// 
        /// This resets but it also checks if the array has changed size due
        /// to creation or destruction of particles
        fn reset_forces(&mut self){
            if self.forces.len() != self.particles.len(){
                self.forces.resize(self.particles.len(), DVec3::ZERO);
                self.torques.resize(self.particles.len(), DVec3::ZERO);
            }else{
                self.forces.fill(DVec3::ZERO);
                self.torques.fill(DVec3::ZERO);
            }
        }
    }
    


fn build_molecule_map(particles: &ParticleVec) -> HashMap<usize, MoleculeData> {
    // group indices by mol_id
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



