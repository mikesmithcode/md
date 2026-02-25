use itertools::izip;
use md_core::particle::{ParticleVec};
use glam::DVec3;

use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::force::Force;
use crate::motion::Move;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SimulationModel{
    Default,
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
pub struct Simulation {
    pub particles: ParticleVec,
    pub settings: SimulationSettings,
    pub current_step: usize,
}

impl Simulation 
    {
    /// Create a new simulation
    pub fn new(particles: ParticleVec, settings: SimulationSettings) -> Self {
        Self {
            particles,
            settings: settings.clone(),
            current_step: settings.start,
        }
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






