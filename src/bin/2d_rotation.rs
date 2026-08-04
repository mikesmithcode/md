//! Explanation of simulation
//! 
//! We have two balls with buried charges. We drop one on the other.


use winit::event_loop::EventLoop;
use glam::DVec3;
use std::collections::HashMap;

// Imports from simulation library
use md::md_sim::{Simulation, SimulationSettings, Forces, Motion, ParticleVec};
use md::md_sim::force::{add_weight,add_granular_collision,add_coulomb};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::particle::MoleculeData;
use md::md_sim::utils::{save_particles, load_latest_particles,  filepaths};
use md::md_viz::Scene;

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

    fn has_internal_forces(&self) -> bool {
        true
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force: DVec3,torque:DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        if particles.ptype[i] == 0{
            force = add_weight(i, force, particles);
        }
        (force, torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3,DVec3){
        (force, torque)=add_granular_collision(i, j, particles, force, torque, settings);
        force = add_coulomb(i,j, particles, force, settings);
        (force, torque)
    }

    fn update_internal_forces(&self, _particles: &ParticleVec, _force: DVec3, _torque: DVec3, _settings: &SimulationSettings){
        
    }
}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings,molecule_map: &HashMap<usize,MoleculeData>, _time:f64) {
        integrate_rigid_bodies(forces, torques, particles, molecule_map, settings);
    }
    fn correct_motion(&self, forces: &[glam::DVec3],  torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize,MoleculeData>) {
        integrate_rigid_bodies_correct(forces,torques,particles, molecule_map, settings)
    }
}



pub fn main() {    

    // Construct filepaths
    let [sim_config_path, scene_config_path, _object_path ,particle_path, video_path] = filepaths(file!());
    
    // load settings
    let sim_settings: SimulationSettings = SimulationSettings::new(&sim_config_path).expect("sim settings not loaded correctly"); 
    
    //-------------------------------------------------------------
    // Create simulation
    //
    // Initialise simulation with bunch of particles from a snapshot file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // Simulation::new() creates the simulation
    // sim.update() to advance the simulation by one step
    // file_io::save_particles(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot"); for data dump.
    //--------------------------------------------------------------
  
    let (particles, start_step, time) = load_latest_particles(&particle_path).expect("Failed to return particles from file");
    //let objects: Vec<ObjectSpec> = load_objects(&object_path).expect("Failed to return objects from file");
    
    //----------------------------------------------------------------------------
    //Initialise the simulation
    //----------------------------------------------------------------------------
    let mut sim= Simulation::new(particles, None, SimUpdate, sim_settings.clone(), time);
    
    //----------------------------------------------------------------
    //  Graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display. Optional video output.
    //  scene.init_headless() for headless video 
    //  Call scene.display() to update window, scene.save_img() to write
    //--------------------------------------------------------------   

    let mut scene: Scene = Scene::from_config(scene_config_path, &sim_settings);  
    let mut event_loop = EventLoop::new(); 
    let _ = scene.view(&event_loop);
    let _ = scene.start_recording(&video_path, start_step);

    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_frame() to write
    // img to file. simulation.update() to advance the simulation by one step
    //--------------------------------------------------------------
    
    println!("Simulation started...");
    
    // Run simulation loop for num_steps
    for step in start_step..=(start_step+sim.settings.num_steps) {

        sim.update();

        // update scene every dump timesteps
        if step % sim.settings.dump == 0 {
            // exit if window close requested
            if scene.poll_events(&mut event_loop) {
                break; 
            }
            
            //Handle graphics
            scene.display(sim.get_particles(), sim.get_objects()).expect("Error updating display");
            //let _ = scene.save_frame(sim.get_particles(), sim.get_objects());

            //save a snapshot of particle positions and if present objects
            save_particles(&particle_path, step, sim.get_particles(), sim.time).expect("Error saving particles to snapshot");
            //save_objects(&snapshot_path, step, sim.get_objects(), sim.time).expect("Error saving objects to snapshot");
        }
        
    }
    scene.close();
    println!("Simulation finished");

}
