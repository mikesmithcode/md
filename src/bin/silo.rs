use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use winit::event_loop::EventLoop;

// Import everything from your md_viz library
use md::md_viz::scene::{Scene, SceneSetup};
use md::md_viz::camera::{Perspective, CameraSettings};
use md::md_viz::objects::SimBox;

// Imports from simulation library
use md::md_sim::simulation::{self, Simulation};
use md::md_sim::simulation::SimulationSettings;
use md::md_sim::force::{Forces, inelastic_collision};
use md::md_sim::motion::Motion;
use md::md_sim::particle::ParticleVec;
use md::md_sim::simulation::SimulationModel;
use md::md_sim::force::{add_weight, zero_forces_ptype};
use md::md_sim::motion::{integrate_verlet_update, integrate_verlet_correct, change_rad};

use md::md_sim::file_io;


pub struct SimUpdate;

impl Forces for SimUpdate{
    fn update_forces(&self, forces: &mut [glam::DVec3], particles: &ParticleVec, settings: &SimulationSettings) {
        
        //Forces which apply to every particle individually
        add_weight(forces, particles);

        //Forces between particles - starting with checking all pairs.
        let n=particles.len();

        for i in 0..n {
            for j in (i + 1)..n {
                if let SimulationModel::Default(collision_params) = &settings.model{
                    inelastic_collision(i, j, particles, forces, collision_params, &settings.sim_box_size);
                }
            }
        }

        zero_forces_ptype(forces, particles, 1);
        zero_forces_ptype(forces, particles, 2);
    }
}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
        integrate_verlet_update(forces, particles, settings);
        change_rad(particles, 0)
    }
    fn correct_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
        integrate_verlet_correct(forces, particles, settings);
    }
}

pub fn main() {    

    //Specify the folder in which all the output will be stored. Assumes in root of workspace.
    const OUTPUT_PATH: &'static str = "output";
    const INPUT_PATH: &'static str = "input";

    //----------------------------------------------------------------
    // Define simulation
    //---------------------------------------------------------------
    let simulation_name = Path::new(file!())
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap();

    let config_filepath = Path::new(INPUT_PATH).join(format!("{}_config.json", simulation_name));
    let snapshot_path = Path::new(OUTPUT_PATH).join("snapshots");
    

    //------------------------------------------------------------
    // Initialise simulation with bunch of particles from a snapshot file and define simulation parameters with a config file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // -----------------------------------------------------------
    
    let (particles, start_step, mut time) = file_io::load_latest_snapshot(&snapshot_path).expect("Failed to return latest snapshot");
    let sim_settings: SimulationSettings = SimulationSettings::new(&config_filepath).expect("sim settings not loaded correctly");
    
    //----------------------------------------------------------------
    //  Define graphics
    //----------------------------------------------------------------


    let scene_settings = SceneSetup {
            camera: CameraSettings{
                perspective: Perspective::Perspective, // Default Perspective::Perspective or Perspective::Orthographic
                window_dt: 0.01,
                headless_dt: 0.01,
                },
            window_size: (640, 480),
            sim_box_setup: SimBox {
                on: true,
                thickness: sim_settings.sim_box_size_f32()[0]/5000.0,
                sim_box_size: sim_settings.sim_box_size_f32(),
            }, 
    };
 

    //-------------------------------------------------------------
    //  Create simulation
    //--------------------------------------------------------------
    
    let mut sim= Simulation::new(particles, SimUpdate, sim_settings.clone());

    //--------------------------------------------------------------
    //  Initialise all graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display
    //  scene.init_headless() for images saved to file
    //  Can run either or none as required. Can't seem to get both to run at present
    //--------------------------------------------------------------   
    
    let mut scene: Scene = Scene::new(scene_settings.clone());
    //let _ = scene.init_headless();
    let mut event_loop = EventLoop::new(); 
    let _ = scene.init_window(&event_loop);

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
            sleep(Duration::from_millis(100));

            //save a snapshot of particle positions etc
            file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), time).expect("Error saving simulation snapshot");
            time += sim.settings.dt;
        }
        
    }
    println!("Simulation finished");

}
