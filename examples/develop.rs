use std::path::Path;
use winit::event_loop::EventLoop;


// Import everything from your md_viz library
use md_viz::scene::{Scene, SceneSetup};
use md_viz::camera::{Perspective, CameraSettings};
use md_viz::objects::SimBox;

use md_sim::Simulation;
use md_sim::simulation::SimulationSettings;
use md_sim::file_io;





pub fn main() {    

    //Specify the folder in which all the output will be stored.
    const OUTPUT_PATH: &'static str = "output";

    //------------------------------------------------------------
    // Initialise simulation with bunch of particles from a snapshot file
    // -----------------------------------------------------------
    let snapshot_path = Path::new(OUTPUT_PATH).join("snapshots");
    let (particles, start_step, mut time) = file_io::load_latest_snapshot(&snapshot_path).expect("Failed to return latest snapshot");

    //----------------------------------------------------------------
    // Define simulation
    //---------------------------------------------------------------
    let sim_settings = SimulationSettings{
        dt: 0.01,
        sim_box_size: [5.0, 5.0, 5.0],
        start: start_step,
        num_steps: 15000,
        sim_path: OUTPUT_PATH,
        dump:100,
    };

    //----------------------------------------------------------------
    //  Define graphics
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
    };


  


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
    
    let _ = scene.init_headless();
    //let mut event_loop = EventLoop::new(); 
    //let _ = scene.init_window(&event_loop);

    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_img() to write
    // img to file. simulation.update() to advance the simulation by one step
    //--------------------------------------------------------------
    
    println!("Simulation started...");
    
    // Run simulation loop for num_steps
    for step in sim_settings.start..=(sim_settings.start+sim_settings.num_steps) {
        simulation.update();

        // update scene every dump timesteps
        if step % sim_settings.dump == 0 {
            // exit if window close requested
            //if scene.poll_events(&mut event_loop) {
            //    break; 
            //}
            
            scene.save_img(&simulation.get_particles(), &OUTPUT_PATH, step).expect("Error saving img"); 
            //scene.camera_control.update_camera(&mut scene.camera);
            //scene.display(&simulation.get_particles()).expect("Error updating display");
            //sleep(Duration::from_millis(100));

            //save a snapshot
            file_io::save_snapshot(&snapshot_path, step, &simulation.get_particles(), time).expect("Error saving simulation snapshot");
            time += sim_settings.dt;
        }
        
    }
    println!("Simulation finished");

}
