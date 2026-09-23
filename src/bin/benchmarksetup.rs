/// Explanation of simulation
/// 
/// This script is used to create the initial condition for the benchmark


use glam::DVec3;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::fs;

// Import everything from your md_viz library


// Imports from simulation library
use md::md_sim::{Forces, Interactivity, Motion, ParticleVec, SimulationSettings};
use md::md_sim::force::{add_directional_weight, add_particle_particle_collision, CollisionParams};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::file_io::{SimulationContext, parse_simulation_args};
use md::md_sim::particle::MoleculeData;



// Used for anything that needs to be used which might change state with simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variables {
    pub up: DVec3,
    pub angle: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
// ForceModel contains a struct associated with each force model used.
pub struct ForceModel{
    pub collision: CollisionParams,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimUpdate {
    pub model: ForceModel,
    #[serde(default)]
    pub variable: Option<Variables>,
}

impl SimUpdate {
    pub fn new(ctx: &SimulationContext) -> Self {
        // 1. Always load the static model config
        let model_file = fs::File::open(&ctx.paths.model_config)
            .unwrap_or_else(|_| panic!("Failed to open {:?}", ctx.paths.model_config));
        let model: ForceModel = serde_json::from_reader(model_file)
            .unwrap_or_else(|e| panic!("Failed to parse model.json: {}", e));

        // 2. Conditionally load variables only if variables.json exists
        let variable = if let Some(ref var_path) = ctx.paths.variables_config {
            let var_file = fs::File::open(var_path)
                .unwrap_or_else(|_| panic!("Failed to open {:?}", var_path));
            let vars: Variables = serde_json::from_reader(var_file)
                .unwrap_or_else(|e| panic!("Failed to parse variables.json: {}", e));
            
            Some(vars)
            
        } else {
            None
        };

        Self { model, variable }
    }
}



impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        true
    }

    fn has_object_forces(&self) -> bool {
        false
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the dynamic particle has weight
        if particles.ptype[i] == 0{
            let var = self.variable.as_ref().expect("Variables are required for this simulation");
            force = add_directional_weight(i, force, particles, var.up);
        }
        (force, _torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        if particles.ptype[i] == 0{
            //Only main particles have granular collisions.
            //println!("possible collide i {}, j {}",particles.ptype[i],particles.ptype[j]); 
            (force, torque)=add_particle_particle_collision(i, j, force, torque,particles, &self.model.collision, settings);
        }
    
        (force, torque)
    }

}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>, _time:f64) {
        integrate_rigid_bodies(forces,torques, particles, molecule_map, settings);
    }

    fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>) {
        integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings);
    }
}


impl Interactivity for SimUpdate {}






pub fn main() {    
    let ctx = parse_simulation_args();
    md::run_simulation(SimUpdate::new(&ctx), ctx);
}
