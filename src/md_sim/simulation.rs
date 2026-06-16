//! The simulation is run by calling methods on the Simulation struct
//! 
//! The Simulation struct is the centrepiece of the simulation. Define one with the ::new() and then
//! update it each timestep with update(). You can get mutable or immmutable references to the particles. The immutable
//! ones can be fed to the visualization.
//! 

use glam::DVec3;

use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::collections::HashMap;
use itertools::izip;

use crate::md_sim::motion::geometry::MoleculeData;
use crate::md_sim::particle::{ParticleVec};
use crate::md_sim::force::neighbours::CellGrid;
use crate::md_sim::force::force::Forces;
use crate::md_sim::motion::motion::Motion;
use crate::md_sim::utils::models::*;





///---------------------------------------------------------
/// These are general rather than particle specific parameters that affect the running of the simulation
/// 
/// 
/// dt - timestep of the simulation
/// sim_box_size - x,y,z dimensions of the simulation box
/// cutoff - range of force or distance within which neighbours are defined by the cell grid / verlet in [`crate::md_sim::neighbours::CellGrid`]
/// skin -  This is the distance beyond the cutoff in which particles are added to a particles verlet list. When any particle travels skin/2 the grid and verlet list are rebuilt.
/// num_steps - How many steps the simulation will advance before stopping
/// dump - Can be used to control how many steps occur before writing to a file or saving an image to the video. But must be used manually in the main loop
/// active_ptypes - A Vec of i32 where each number represents. 
/// 
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: DVec3, 
    pub cutoff: f64,
    pub skin: f64,
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    pub interaction_ptypes: Vec<[u8;2]>,
    //#[serde(default)]
    //pub head_ptypes: Vec<u8>,
    pub model: SimulationModel,
    #[serde(skip)] // Don't try to load this from JSON
    pub active_mask: [bool; 32],
    
}

impl SimulationSettings {
    /// Loads sim config from file and builds the active mask
    pub fn new(path: &Path) -> Result<SimulationSettings, Box<dyn std::error::Error>> {
        let file = File::open(path).map_err(|e| {
            format!(
                "\n==========================================\n\
                Error: Couldn't find config at {}\n\
                Details: {}\n\
                ==========================================\n", 
                path.display(), e
            )
        })?;
        
        let reader = BufReader::new(file);
        let mut sim_settings: SimulationSettings = serde_json::from_reader(reader)?;

        // Build the active mask from interaction_ptypes
        // A ptype is active if it appears as the first element in any pair
        sim_settings.active_mask = [false; 32];
        for pair in &sim_settings.interaction_ptypes {
            let ptype = pair[0] as usize;
            if ptype < 32 {
                sim_settings.active_mask[ptype] = true;
            }
        }

        Ok(sim_settings)
    }

    pub fn sim_box_size_f32(&self) -> [f32; 3] {
        self.sim_box_size.as_vec3().to_array()
    }

    /// Helper to check if a type should have forces calculated
    #[inline]
    pub fn is_active(&self, ptype: usize) -> bool {
        //check bounds if we only use 32
        ptype < 32 && self.active_mask[ptype]
    }

    //pub fn is_head(&self, ptype: u8) -> bool {
    //    self.head_ptypes.contains(&ptype)
    //}
}

/// Largely used for testing
impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            dt: 0.1,
            sim_box_size: DVec3::new(10.0, 0.1, 10.0),
            cutoff: 1.0,
            skin:0.2,
            start: 0,
            num_steps: 15,
            dump: 1000,
            interaction_ptypes: vec![[0,0]],
            //head_ptypes: vec![],
            model: SimulationModel::Solid(CollisionParams{
                stiffness: 1000.0, 
                damping: 50.0}),
            active_mask:[true;32]
        }

    }
}


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
    pub forces: Vec<DVec3>,
    pub torques: Vec<DVec3>,
    pub sim_update: S,
    pub settings: SimulationSettings,
    pub current_step: usize,
    pub cell_grid: CellGrid,
    pub time: f64,
    pub molecule_map: HashMap<usize, MoleculeData>,
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

impl<S> Simulation<S> 
    where 
        S: Forces + Motion,
    {
        /// Create a new simulation
        pub fn new(particles: ParticleVec, sim_update: S, settings: SimulationSettings, time: f64) -> Self {
            let n = particles.len();
            let molecule_map = build_molecule_map(&particles);


            Self {
                particles,
                forces : vec![DVec3::ZERO; n],
                torques : vec![DVec3::ZERO; n],
                sim_update,
                settings: settings.clone(),
                current_step: settings.start,
                cell_grid: CellGrid::new(settings.sim_box_size,settings.cutoff,n, settings.skin),
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
                for i in 0..self.particles.len(){
                    if self.settings.is_active(self.particles.ptype[i]){
                        self.sim_update.update_single_forces(i, &mut self.forces, &mut self.torques, &self.particles, &self.settings, self.time);
                    }
                }
            }

            // Grid means you only check particles nearby. Then it calculates all pair forces 
            // between particles i and j.
            if self.sim_update.has_pair_forces(){
                //Check if grid and verlet lists need recalculating
                self.cell_grid.check_and_rebuild_neighbours(&mut self.particles, &self.settings);
                //appy pairwise forces
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

        //Return read only reference to particles
        pub fn get_particles(&self)-> &ParticleVec{
            &self.particles
        }

        //Return mut reference to particles
        pub fn get_mut_particles(&mut self)-> &mut ParticleVec{
            &mut self.particles
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
    






