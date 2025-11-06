pub mod scene; // Declare the 'scene' module
pub mod shapes;
pub mod objects;
pub mod video;
pub mod camera;

// Re-export common types and functions for easier consumption
pub use crate::scene::*;
pub use crate::objects::*;
pub use crate::shapes::*;


//use three_d::FrameOutput;
/*use md_sim::simulation::Simulation;


///Used to display live animation whilst running simulation.
pub fn run_animated_simulation(mut simulation: Simulation, scene_settings: SceneSetup) {

    let window = create_window(scene_settings.window_size);
    let mut scene = Scene::new(&window, scene_settings);

    window.render_loop(move |mut frame_input| {
        let mut accumulator = simulation.settings.dt;
        accumulator += frame_input.elapsed_time;

        while accumulator >= simulation.settings.dt {
            simulation.update();
            accumulator -= simulation.settings.dt;
        }

        let particles_data = simulation.get_particles();
        scene.render(&mut frame_input, &particles_data);

        FrameOutput::default()
    });
}
*/
