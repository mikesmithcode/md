
use std::path::Path;
use winit::event_loop::EventLoop;
use glam::DVec3;

// Import everything from your md_viz library
use md::md_viz::scene::{Scene, SceneSetup};
use md::md_viz::camera::CameraView;
use md::md_viz::objects::SimBox;

// Imports from simulation library
use md::md_sim::simulation::Simulation;
use md::md_sim::simulation::SimulationSettings;
use md::md_sim::force::{Forces, inelastic_collision};
use md::md_sim::motion::Motion;
use md::md_sim::particle::ParticleVec;
use md::md_sim::force::{add_weight, zero_forces_for_ptypes};
use md::md_sim::motion::{integrate_verlet_update, integrate_verlet_correct, move_sinwave, change_colour};

use md::md_sim::file_io;
use md::md_sim::simulation::SimulationModel;
use md::md_sim::force::check_delta;



pub struct SimUpdate;

pub fn add_surface(i: usize, forces: &mut [DVec3], particles: &ParticleVec, settings: &SimulationSettings){
    let height: f64 = 0.005;
    let SURFACE_NORMAL = DVec3::new(0.0, 0.0, 1.0);

    let (stiffness, damping) = match &settings.model{
        SimulationModel::Solid(params) => {
            (params.stiffness, params.damping)
            }
        _ => panic!("Settings model must use the Solid enum")
    };

    let rad = particles.radius[i];
    let z = particles.position[i].z;
    let dz = z - height;
    let side = dz.signum();
    let contact_normal = SURFACE_NORMAL * side;
    let overlap = rad - dz.abs();

    if overlap > 0.0 {        
        // Relative Velocity and Normal Component
        let rel_vel = particles.velocity[i];
        let normal_vel = rel_vel.dot(contact_normal);

        // Force Calculation (Spring + Damping)
        let spring_f= stiffness * overlap;
        let damping_f = -damping * normal_vel;

        // Ensure total force is never attractive (clamping)
        let total_f = (spring_f + damping_f).max(0.0);
        let f_vec = contact_normal * total_f;

        forces[i] += f_vec;
    }


}

impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        false
    }
    
    fn update_pair_forces(
            &self, 
            i: usize, 
            j: usize, 
            forces: &mut [DVec3], 
            particles: &ParticleVec, 
            settings: &SimulationSettings
        ) {
        
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, forces: &mut [glam::DVec3], particles: &ParticleVec, settings: &SimulationSettings, time:f64) {   
        add_weight(i, forces, particles);
        add_surface(i, forces, particles, settings);

    }

}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, time:f64) {
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
    
    let (particles, start_step, mut time) = file_io::load_latest_snapshot(&snapshot_path).expect("Failed to return latest snapshot");

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
            scene.display(&sim.get_particles()).expect("Error updating display");
            let _ = scene.save_frame(&sim.get_particles());
            //sleep(Duration::from_millis(100));

            //save a snapshot of particle positions etc
            {
                file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot");

            }
        }
        
    }
    println!("Simulation finished");

}
