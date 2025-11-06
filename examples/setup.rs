use glam::DVec3;
use three_d::core::Srgba;
use std::time::Duration;
use std::thread::sleep;
use winit::event_loop::EventLoop;


// Import everything from your md_viz library
use md_viz::scene::{Scene, SceneSetup};
use md_viz::objects::{Perspective, CameraSettings};
use md_viz::shapes::SimBox;
use md_viz::video::assemble_pngs_to_mp4;
use md_sim::Simulation;
use md_sim::simulation::SimulationSettings;

// Import the Particle and Simulation from your simulation crate
use md_core::particle::Particle;




pub fn main() {    
    //----------------------------------------------------------------
    // Define simulation
    //---------------------------------------------------------------
    let sim_settings = SimulationSettings{
        dt: 0.01,
        sim_box_size: [5.0, 5.0, 5.0],
        start: 0,
        stop: 1000,
        sim_filename: String::from("test/test"),
    };

    //----------------------------------------------------------------
    //  Define grpahics
    //----------------------------------------------------------------

    let scene_settings = SceneSetup {
            camera: CameraSettings{
                perspective: Perspective::Orthographic, // Default Perspective::Perspective or Perspective::Orthographic
                window_dt: 0.01,
                headless_dt: 0.01,
                },
            window_size: (640, 480),
            sim_box_setup: SimBox {
                on: true,
                thickness: 0.1,
                sim_box_size: sim_settings.sim_box_size_f32(),
            },
            img_filepath: sim_settings.sim_filename.clone(),   
    };


    //------------------------------------------------------------
    // Initialise simulation with bunch of particles
    // -----------------------------------------------------------
    let particles = vec![
        Particle::new(
            0,
            DVec3::new(1.5, 1.5, 1.5),
            DVec3::new(0.0, 0.02, 0.0),
            Srgba::new(255, 0, 0, 255), // Red
            0.5,
        ),
        Particle::new(
            1,
            DVec3::new(1.5, -1.5, -1.5),
            DVec3::new(0.0, 0.003, 0.0),
            Srgba::new(0, 255, 0, 255), // Green
            0.5,
        ),
        Particle::new(
            2,
            DVec3::new(1.5, -1.5, 1.5),
            DVec3::new(0.0, 0.01, 0.0),
            Srgba::new(0, 0, 255, 255), // Blue
            0.5,
        ),
    ];




    //-------------------------------------------------------------
    //  Create simulation
    //--------------------------------------------------------------
    let mut simulation = Simulation::new(particles, sim_settings.clone());

    //--------------------------------------------------------------
    //  Initialise all graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display
    //  scene.init_headless() for images saved to file
    //  Can run either or none as required. Can't seem to get both to run at present
    //--------------------------------------------------------------
      
    
    let mut scene: Scene = Scene::new(scene_settings.clone());
    
    //let _ = scene.init_headless();
    let event_loop = EventLoop::new(); 
    let _ = scene.init_window(&event_loop);

    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_img() to write
    // img to file. simulation.update() to advance the simulation by one step
    //--------------------------------------------------------------
    
    println!("Simulation started...");
    for i in sim_settings.start..sim_settings.stop {
        simulation.update();
        if i % 100 == 0 {
            
            //scene.save_img(&simulation.get_particles(), &scene_settings.img_filepath, i).expect("Error saving img"); 
            scene.display(&simulation.get_particles()).expect("Error updating display");
            sleep(Duration::from_millis(100));
        }
        
    }
    println!("Simulation finished");

    // Assemble images into movie as required upon completion using ffmpeg.
    //assemble_pngs_to_mp4(scene_settings.img_filepath).expect("Video writing failed");
}
