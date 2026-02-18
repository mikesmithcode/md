use md_core::particle::Particle;
use serde_json::Error;
use std::collections::HashMap;
use serde_json::Value;
use glam::DVec3;
//use crate::file_io::{load_simsettings, save_simsettings};

//use glam::DVec3;
//use three_d::core::Srgba;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::forces::{Forces, calc_drag, calc_gravity};

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

impl<T> SimulationSettings<T>
    {
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


///Types of simulation_settings
/// Unit struct to be passed to load_simsettings if nothing beyond default params in file.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NoExtraParams;

pub trait Fluid{
    fn viscosity(&self) -> f64;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FluidParams {
    pub viscosity: f64,
}

impl Fluid for FluidParams{
    fn viscosity(&self)-> f64{
        self.viscosity
    }
}







/// The main simulation engine
#[derive(Clone, Debug)]
pub struct Simulation<T> {
    pub particles: Vec<Particle>,
    pub settings: SimulationSettings<T>,
    pub current_step: usize,
}

impl<T> Simulation<T> 
    where 
        T: Clone + Forces
    {
    /// Create a new simulation
    pub fn new(particles: Vec<Particle>, settings: SimulationSettings<T>) -> Self {
        Self {
            particles,
            settings: settings.clone(),
            current_step: settings.start,
        }
    }

    pub fn update(&mut self){
        let mut forces = vec![DVec3::ZERO; self.particles.len()];
        self.settings.extra.update_forces(&self.particles, &mut forces);
        self.update_motion(&forces);
    }

    /// Update motion of particles by applying forces and stepping forward one dt
    pub fn update_motion(&mut self, forces: &[DVec3]) {
        let dt = self.settings.dt;
        for (idx,particle) in self.particles.iter_mut().enumerate(){
            particle.velocity += forces[idx]*particle.inv_mass * dt;
            particle.position += particle.velocity * dt;
            
            //Apply periodic boundaries
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

/// Simulation<NoExtraParams> When the simulation has no extra params
impl Simulation<NoExtraParams> {
    pub fn update_forces(&mut self, forces: &mut [DVec3]) {
        // Only gravity is available here
        calc_gravity(&self.particles, forces);
    }
}

///Simlation<FluidParams> When the simulation has fluid params
impl Simulation<FluidParams> {
    pub fn update_forces(&mut self, forces: &mut [DVec3]) {
        // Gravity + Drag
        calc_gravity(&self.particles, forces);
        calc_drag(&self.particles, forces, &self.settings.extra);
    }
}






pub fn check_periodic(pos: &mut DVec3, sim_box_size: DVec3){
        *pos = *pos - sim_box_size * (*pos / sim_box_size).floor();
    }
