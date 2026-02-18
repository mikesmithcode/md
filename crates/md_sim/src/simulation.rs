use md_core::particle::Particle;
use serde_json::Error;
use glam::DVec3;
//use crate::file_io::{load_simsettings, save_simsettings};

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
    pub sim_box_size: DVec3, 
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
        let file = File::open(path).unwrap_or_else(|_err| {
            panic!("\n==========================================\nError: Couldn't find file at {}\n==========================================\n", path.display());
        });
        let reader = BufReader::new(file);

        let sim_settings = serde_json::from_reader::<_, SimulationSettings<T>>(reader)?;

        Ok(sim_settings)
    }

    pub fn sim_box_size_f32(&self)->[f32;3]{
        self.sim_box_size.as_vec3().to_array()
    }
}



/// The main simulation engine
#[derive(Clone, Debug)]
pub struct Simulation<T> {
    pub particles: Vec<Particle>,
    pub settings: SimulationSettings<T>,
    pub current_step: usize,
}

impl<T: Clone> Simulation<T> {
    /// Create a new simulation
    pub fn new(particles: Vec<Particle>, settings: SimulationSettings<T>) -> Self {
        Self {
            particles,
            settings: settings.clone(),
            current_step: settings.start,
        }
    }

    pub fn update_force(&mut self){
        println!("forces");
    }

    /// Run one simulation step
    pub fn update_pos(&mut self) {
        // Your simulation logic here
        
        for particle in self.particles.iter_mut(){
            println!("{:?}", particle.position);
            particle.position.x += 0.012;
            particle.position.y += 0.002;
            check_periodic(&mut particle.position, self.settings.sim_box_size);
        }
    }


    pub fn update_rad(&mut self){
        println!("radius");
    }

    pub fn get_particles(&self)-> &[Particle]{
        &self.particles
    }
}


pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3){
        *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
    }
