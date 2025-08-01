// Import everything from your md_viz library
use md_viz::scene::{self, Scene, SceneSetup};
use md_viz::objects::{Perspective, CameraSettings, create_window};
use md_viz::shapes::SimBox;
use md_viz::run_animated_simulation;

use md_sim::{Simulation, SimulationSettings, run_headless_simulation};



pub fn main() {    
    let sim_settings = SimulationSettings{
        dt: 0.0001,
        sim_box_size: [5.0, 5.0, 5.0],
    };

    let scene_settings = SceneSetup {
            camera: CameraSettings{
                perspective: Perspective::Orthographic, // Default perspective
                },
            window_size: (640, 480),
            sim_box: SimBox {
                on: true,
                thickness: 0.2,
                sim_box_size: sim_settings.sim_box_size_f32(),
            },  
        };

    let simulation = Simulation::new(sim_settings);

    run_animated_simulation(simulation, scene_settings);
    //run_headless_simulation(simulation, 100);

}
