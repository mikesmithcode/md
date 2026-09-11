/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use winit::event_loop::EventLoop;
use glam::DVec3;
use std::collections::HashMap;

// Import everything from your md_viz library
use md::md_viz::{init_scene, scene::Scene};
use md::md_viz::scene_settings::SceneSettings;

// Imports from simulation library
use md::md_sim::{Forces, Motion, ObjectSpec, ParticleVec, Simulation, SimulationSettings};
use md::md_sim::force::{add_coulomb, add_particle_object_collision, add_particle_particle_collision, add_weight};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::{parse_simulation_args, save_particles, load_latest_particles, load_latest_objects, SimulationPaths};
use md::md_sim::particle::MoleculeData;



pub struct SimUpdate;

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
        true
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the main particle has weight
        if particles.ptype[i] == 0 || particles.ptype[i] == 2{
            force = add_weight(i, force, particles);
        }
        (force, _torque)
    }

    fn update_object_forces(&self, i: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec, objects: &ObjectSpec, settings: &SimulationSettings)->(DVec3, DVec3){
        //Only main particle collides with the surface
        if particles.ptype[i] == 0 || particles.ptype[i] == 2{
           (force,torque) = add_particle_object_collision(i, particles, objects, force, torque, settings);
        }
        (force, torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        // guaranteed that i and j will be same ptype due to verlet list specs
        if particles.ptype[i] == 0 || particles.ptype[i] == 2{
            //Only main particles have granular collisions. 
            (force, torque)=add_particle_particle_collision(i, j, particles, force, torque, settings);
        }
        else{
            //ptype == 1 is the charge.
            force = add_coulomb(i, j, particles, force, settings);
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

    fn update_objects(&self, object: &mut ObjectSpec, _particles: &mut ParticleVec, settings: &SimulationSettings, time: f64){
        match object {
            ObjectSpec::Rectangle(rect) => {
                let ang_freq = 500.0;
                let velocity = DVec3::new(0.0,0.0,1.0)*f64::sin(ang_freq*time);
                rect.step(velocity, DVec3::ZERO, settings.dt);
            },
            _ => {}   
        }
    }

}




pub fn main() {    
    md::run_simulation(SimUpdate);
}