use md_core::particle::Particle;
use serde_json::Error;
use glam::DVec3;
use crate::file_io::{load_simsettings, save_simsettings};

//use glam::DVec3;
//use three_d::core::Srgba;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
///---------------------------------------------------------
///Simulation settings 
/// 
/// These are parameters that affect the running of the simulation such as time step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings<T>{
    pub dt: f64,
    pub sim_box_size: [f64; 3], 
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    // Special values
    #[serde(flatten)]
    pub extra: T,
}

impl<T> SimulationSettings<T>{
    /// loads both sim config and initial state from file
    /// 
    /// Path 
    pub fn new(path: &Path)-> Result<SimulationSettings<T>, Box<dyn std::error::Error>>
        where T: DeserializeOwned + Serialize,
    {
        let file = File::open(path).expect("Error opening config file");
        let reader = BufReader::new(file);

        let sim_settings = serde_json::from_reader::<_, SimulationSettings<T>>(reader)?;

        Ok(sim_settings)
    }

    pub fn sim_box_size_f32(&self)->[f32;3]{
        [self.sim_box_size[0] as f32,self.sim_box_size[1] as f32,self.sim_box_size[2] as f32]
    }
}

