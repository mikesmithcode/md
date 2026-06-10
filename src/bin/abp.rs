
/// Explanation of simulation
/// 
/// Demo for active brownian particles



use winit::event_loop::EventLoop;
use glam::DVec3;


// Imports from simulation library
use md::md_sim::file_io;
use md::md_sim::simulation::Simulation;
use md::md_sim::simulation::SimulationSettings;
use md::md_sim::force::{Forces, weeks_chandler_andersen, active_force};
use md::md_sim::motion::{Motion,update_abps};
use md::md_sim::particle::ParticleVec;


use md::md_viz::scene::Scene;


pub struct SimUpdate;

impl Forces for SimUpdate{

    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, forces: &mut [glam::DVec3], _torques: &mut [DVec3], particles: &ParticleVec, settings: &SimulationSettings, _time: f64) {   
       active_force(i, forces, particles, settings);
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize,forces: &mut [DVec3], _torques: &mut [DVec3], particles: &ParticleVec,settings: &SimulationSettings){
        weeks_chandler_andersen(i, j, forces, particles, settings);
    }

}



/// Add any changes to the motion e.g particles changing size, being created or disappearing. Then integrate the equations of motion.
impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[DVec3], _torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, _time:f64) {
        if forces.iter().all(|&f| f == DVec3::ZERO) {
            return;
        }
                 update_abps(forces, particles, settings);
            
       
    }
}




pub fn main() {    

    // Construct filepaths from script name.
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
    //  Graphics are handled by a Scene struct.
    //  For any graphics you need an event_loop and scene.view() for live display or scene.background() for hidden. 
    //  Optional video output requires you to scene.start_recording()
    //
    //  Once in your main loop you can update display with scene.display() and add a frame to the video with scene.save_frame()
    //--------------------------------------------------------------   

    let mut scene: Scene = Scene::from_config(scene_config_path, &sim_settings);   
    let mut event_loop = EventLoop::new(); 
    let _ = scene.view(&event_loop);
    //let _ = scene.background(&event_loop);
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
