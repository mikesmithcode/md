/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use winit::event_loop::EventLoop;
use glam::DVec3;


// Imports from simulation library
use md::md_sim::file_io;
use md::md_sim::simulation::Simulation;
use md::md_sim::simulation::SimulationSettings;
use md::md_sim::force::{Forces, inelastic_collision};
use md::md_sim::motion::Motion;
use md::md_sim::particle::ParticleVec;
use md::md_sim::force::{add_weight, zero_forces_for_ptypes};
use md::md_sim::motion::{integrate_verlet_update, integrate_verlet_correct};

use md::md_viz::scene::Scene;

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


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, forces: &mut [glam::DVec3], particles: &ParticleVec, _settings: &SimulationSettings, _time: f64) {   
        add_weight(i, forces, particles);
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize,forces: &mut [DVec3],particles: &ParticleVec,settings: &SimulationSettings){
        inelastic_collision(i, j, particles, forces, settings);
    }

    // For particles that shouldn't follow the calculated forces e.g walls etc.
    fn update_ptype_no_forces(&self, forces: &mut [DVec3], particles: &ParticleVec){
        let immobile = &[1, 2];
        zero_forces_for_ptypes(forces, particles, immobile);
    }
}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, _time:f64) {
        integrate_verlet_update(forces, particles, settings);
    }
    fn correct_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
        integrate_verlet_correct(forces, particles, settings);
    }
}



pub fn main() {    

    // Construct filepaths
    let [sim_config_path, scene_config_path, snapshot_path, video_path] = file_io::filepaths(file!());
    

    //------------------------------------------------------------
    // Initialise simulation with bunch of particles from a snapshot file and define simulation parameters with a config file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // -----------------------------------------------------------
    
    let (_particles, start_step, mut _time) = file_io::load_latest_snapshot(&snapshot_path).expect("Failed to return latest snapshot");

    // load settings
    let sim_settings: SimulationSettings = SimulationSettings::new(&sim_config_path).expect("sim settings not loaded correctly"); 

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

    //-------------------------------------------------------------
    // Create simulation
    //
    // Initialise simulation with bunch of particles from a snapshot file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // Simulation::new() creates the simulation
    // sim.update() to advance the simulation by one step
    // file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot"); for data dump.
    //--------------------------------------------------------------
  
    let (particles, start_step, time) = file_io::load_latest_snapshot(&snapshot_path).expect("Failed to return latest snapshot");
    let mut sim= Simulation::new(particles, SimUpdate, sim_settings.clone(), time);
    
    println!("Simulation started...");
    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_img() to write
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
            //scene.save_img(&sim.get_particles(), &OUTPUT_PATH, step).expect("Error saving img"); 
            scene.display(&sim.get_particles()).expect("Error updating display");
            let _ = scene.save_frame(&sim.get_particles());

            //save a snapshot of particle positions etc
            file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot");
        }
        
    }
    scene.close();
    println!("Simulation finished");

}
