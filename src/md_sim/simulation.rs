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

use crate::md_sim::particle::{ParticleVec};
use crate::md_sim::neighbours::CellGrid;
use crate::md_sim::force::Forces;
use crate::md_sim::motion::Motion;


/// SimulationModel defines the structure of the file to be read in which may be different in different simulations
/// 
/// The json tells serde what variant it should use.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SimulationModel{
    Solid(CollisionParams),
    Fluid {viscosity: f64, cutoff: f64},
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionParams {
    pub stiffness: f64,
    pub damping: f64,
}

///---------------------------------------------------------
/// These are general rather than particle specific parameters that affect the running of the simulation
/// 
/// 
/// dt - timestep of the simulation
/// sim_box_size - x,y,z dimensions of the simulation box
/// cutoff - range of force or distance within which neighbours are defined by the cell grid / verlet in [`neighbours::CellGrid`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: DVec3, 
    pub cutoff: f64,
    pub skin: f64,
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    pub active_ptypes: Vec<i32>,
    pub model: SimulationModel,
    #[serde(skip)]  
    pub active_map: [bool; 32],
    
}

impl SimulationSettings
    {
    /// loads both sim config and initial state from file
    /// 
    /// Path
    pub fn new(path: &Path)-> Result<SimulationSettings, Box<dyn std::error::Error>>
    {
        let file = File::open(path).unwrap_or_else(|_err| {
            panic!("\n==========================================\nError: Couldn't find file at {}\n==========================================\n", path.display());
        });
        let reader = BufReader::new(file);

        let mut sim_settings = serde_json::from_reader::<_, SimulationSettings>(reader)?;
        sim_settings.active_map = [false; 32];
        for &ptype in &sim_settings.active_ptypes {
            if ptype >= 0 && (ptype as usize) < 32 {
                sim_settings.active_map[ptype as usize] = true;
            }
        }

        Ok(sim_settings)
    }

    pub fn sim_box_size_f32(&self)->[f32;3]{
        self.sim_box_size.as_vec3().to_array()
    }
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
            active_ptypes: vec![0],
            model: SimulationModel::Solid(CollisionParams{
                stiffness: 1000.0, 
                damping: 50.0}),
            active_map:[true;32]
        }

    }
}


/// The main simulation engine
#[derive(Debug)]
pub struct Simulation<S> 
    where 
        S: Forces + Motion,
{
    pub particles: ParticleVec,
    pub forces: Vec<DVec3>,
    pub sim_update: S,
    pub settings: SimulationSettings,
    pub current_step: usize,
    pub cell_grid: CellGrid,
    pub time: f64,
}

impl<S> Simulation<S> 
    where 
        S: Forces + Motion,
    {
    /// Create a new simulation
    pub fn new(particles: ParticleVec, sim_update: S, settings: SimulationSettings, time: f64) -> Self {
        let n = particles.len();
        Self {
            particles,
            forces : vec![DVec3::ZERO; n],
            sim_update,
            settings: settings.clone(),
            current_step: settings.start,
            cell_grid: CellGrid::new(settings.sim_box_size,settings.cutoff,n, settings.skin),
            time
        }
    }

    /// Update the simulation
    /// 
    /// The positions and velocities are updated in 2 steps
    /// First we predict the motion based on current values
    /// Then we calculate the forces
    /// Then we correct our prediction in light of the new forces.
    /// Only pairs of particles within the cutoff distance are 
    /// calculated for the pair forces.
    pub fn update(&mut self){

        // Predict the new positions, velocities etc
        self.sim_update.update_motion(&self.forces, &mut self.particles, &self.settings, self.time);


        // Form the cell_grid, putting particles in
        if self.sim_update.has_single_forces() || self.sim_update.has_pair_forces(){
            //Clear the force buffer and check same length as particles
            self.reset_forces();
        }

        if self.sim_update.has_single_forces(){
            // Single forces apply to individual particles
            for i in 0..self.particles.len(){
                self.sim_update.update_single_forces(i, &mut self.forces, &self.particles, &self.settings, self.time);
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
                &self.particles, 
                &self.sim_update, 
                &self.settings
                );
        }

        // Prevents some particles from responding to the forces eg walls.
        self.sim_update.update_ptype_no_forces(&mut self.forces, &self.particles);


        // Perform correction to the motion based on the updated forces
        self.sim_update.correct_motion(&self.forces, &mut self.particles, &self.settings);

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
        }else{
            self.forces.fill(DVec3::ZERO);
        }
    }
}






