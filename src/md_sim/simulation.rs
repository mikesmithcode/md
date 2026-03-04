use glam::DVec3;

use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::md_sim::particle::{ParticleVec};
use crate::md_sim::force::Forces;
use crate::md_sim::motion::Motion;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SimulationModel{
    Default(CollisionParams),
    Fluid {viscosity: f64, cutoff: f64},
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionParams {
    pub stiffness: f64,
    pub damping: f64,
}

///---------------------------------------------------------
///Simulation settings 
/// 
/// These are parameters that affect the running of the simulation such as time step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: DVec3, 
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    pub model: SimulationModel,
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

        let sim_settings = serde_json::from_reader::<_, SimulationSettings>(reader)?;

        Ok(sim_settings)
    }

    pub fn sim_box_size_f32(&self)->[f32;3]{
        self.sim_box_size.as_vec3().to_array()
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
}

impl<S> Simulation<S> 
    where 
        S: Forces + Motion,
    {
    /// Create a new simulation
    pub fn new(particles: ParticleVec, sim_update: S, settings: SimulationSettings) -> Self {
        let n = particles.len();
        Self {
            particles,
            forces : vec![DVec3::ZERO; n],
            sim_update,
            settings: settings.clone(),
            current_step: settings.start,
        }
    }

    pub fn update(&mut self){

        self.sim_update.update_motion(&self.forces, &mut self.particles, &self.settings);

        //Clear the force buffer and check same length as particles
        if self.forces.len() != self.particles.len(){
            self.forces.resize(self.particles.len(), DVec3::ZERO);
        }else{
            self.forces.fill(DVec3::ZERO);
        }


        self.sim_update.update_forces(&mut self.forces, &self.particles, &self.settings);
        self.sim_update.correct_motion(&self.forces, &mut self.particles, &self.settings);
        
    }

    //Return read only reference to particles
    pub fn get_particles(&self)-> &ParticleVec{
        &self.particles
    }

    //Return mut reference to particles
    pub fn get_mut_particles(&mut self)-> &mut ParticleVec{
        &mut self.particles
    }
}






