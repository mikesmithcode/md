use itertools::izip;
use md_core::particle::{ParticleVec};
use glam::DVec3;
//use crate::file_io::{load_simsettings, save_simsettings};

//use glam::DVec3;
//use three_d::core::Srgba;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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

    pub fn update(&mut self){
        let mut forces = vec![DVec3::ZERO; self.particles.len()];
        self.update_motion(&forces);
    }

    /// Update motion of particles by applying forces and stepping forward one dt
    pub fn update_motion(&mut self, forces: &[DVec3]) {
        let dt = self.settings.dt;
        let box_size = self.settings.sim_box_size;

        // We zip the columns of the ParticleVec along with the external forces slice
        for (pos, vel, inv_mass, force) in izip!(
            &mut self.particles.position,
            &mut self.particles.velocity,
            &self.particles.inv_mass,
            forces
        ) {
            let acceleration = *force * (*inv_mass);
            *vel += acceleration * dt;
            *pos += *vel * dt;
            
            // Apply periodic boundaries
            check_periodic(pos, box_size);
        }
    }

    //Return read only reference to particles
    pub fn get_particles(&self)-> &ParticleVec{
        &self.particles
    }
}





pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3){
        *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
    }
