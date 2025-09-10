// Import everything from your md_viz library
use md_viz::scene::{Scene, SceneSetup};
use md_viz::objects::{Perspective, CameraSettings};
use md_viz::shapes::SimBox;
use md_sim::Simulation;
use md_sim::simulation::SimulationSettings;



pub fn main() {    

    let sim_settings = SimulationSettings{
        dt: 0.0001,
        sim_box_size: [5.0, 5.0, 5.0],
        start: 0,
        stop: 10000,
    };

    let scene_settings = SceneSetup {
            camera: CameraSettings{
                perspective: Perspective::Orthographic, // Default Perspective::Perspective or Perspective::Orthographic
                dt_frame: 0.001,
                },
            window_size: (640, 480),
            sim_box: SimBox {
                on: true,
                thickness: 0.1,
                sim_box_size: sim_settings.sim_box_size_f32(),
            },  
        };

    let mut simulation = Simulation::new(sim_settings.clone());
    let mut scene: Scene = Scene::new(&scene_settings);

    scene.init_headless(&scene_settings);

    //run_animated_simulation(simulation, scene_settings);
    
    println!("Starting headless simulation...");
    for i in sim_settings.start..sim_settings.stop {
        simulation.update();
        if i % 1000 == 0 {
            println!("Simulating step {}", i);
            if let Err(e) = scene.save_img(&simulation.get_particles(), &format!("test/test{:04}.png", i)) {
    eprintln!("Failed to save image: {}", e);
}
        }
    }
    println!("Headless simulation finished");
}



