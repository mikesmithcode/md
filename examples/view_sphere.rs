use three_d::*; // Still need three_d for basic types like Window, Context, Viewport
use md_sim::Simulation;

// Import everything from your md_viz library
use md_viz::scene::{Scene, create_window};









pub fn main() {
    let num_particles = 100;
    let sim_box_size_f64: [f64;3] = [10.0,10.0,20.0];  
    let window_size:(u32,u32) = (500, 500);


    let sim_box_size_f32: [f32; 3] = sim_box_size_f64.map(|x| x as f32);
    let window = create_window(window_size); // Use the function from your lib
    let context = window.gl();

    let simulation = Simulation::new(num_particles, sim_box_size_f64);

    // Create the scene using the Scene::new constructor from your lib
    let scene = Scene::new(&context, window.viewport(), sim_box_size_f32, num_particles);

    // Run the application loop
    scene.run(window, simulation);
}