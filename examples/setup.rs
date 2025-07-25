use three_d::*; // Still need three_d for basic types like Window, Context, Viewport
use md_sim::Simulation;

// Import everything from your md_viz library
use md_viz::scene::{Scene, SceneSetup};
use md_viz::objects::{Perspective, CameraSettings, create_window};
use md_viz::shapes::{SimBox};



pub fn main() {
    let scene_settings = SceneSetup {
            camera: CameraSettings{
                perspective: Perspective::Orthographic, // Default perspective
                },
                window_size: (640, 480),
                sim_box: SimBox {
                    on: true,
                    sim_box_size: [10.0, 20.0, 10.0],
                },  
        };
  
    //let sim_box_size_f64: [f64; 3] = SceneSetup.sim_box_size.map(|x| x as f64);
    let window = create_window(scene_settings.window_size); // Use the function from your lib

    //let simulation = Simulation::new(sim_box_size_f64);

    // Create the scene using the Scene::new constructor from your lib
    //let scene = Scene::new(&context, window.viewport(), sim_box_size_f32);//, num_particles);
    let scene = Scene::new(&window, scene_settings);//, num_particles);

    // Run the application loop
    scene.run(window);//, simulation);
}
