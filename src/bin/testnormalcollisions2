/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use winit::event_loop::EventLoop;
use glam::DVec3;
use std::collections::HashMap;

// Import everything from your md_viz library
use md::md_viz::scene::Scene;
use md::md_viz::scene_settings::SceneSettings;

// Imports from simulation library
use md::md_sim::{Interactivity, Forces, Motion, ParticleVec, Simulation, SimulationSettings};
use md::md_sim::force::{add_particle_particle_collision};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::{parse_simulation_args, save_particles, load_latest_particles};
use md::md_sim::particle::MoleculeData;



pub struct SimUpdate;

impl Interactivity for SimUpdate{}

impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        false
    }

    fn has_object_forces(&self) -> bool {
        false
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the main particle has weight
        (force, _torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize,mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        // guaranteed that i and j will be same ptype due to verlet list specs
        
        //main particles have granular collisions. 
        (force, torque)=add_particle_particle_collision(i, j, particles, force, torque, settings);

       
        
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




pub fn main() {    

    md::run_simulation(SimUpdate);

}
