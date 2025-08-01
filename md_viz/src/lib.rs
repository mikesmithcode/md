pub mod scene; // Declare the 'scene' module
pub mod shapes;
pub mod objects;

// Re-export common types and functions for easier consumption
// You might want to be more selective about what you re-export.
pub use crate::scene::*;
pub use crate::objects::*;
pub use crate::shapes::*;

use three_d::FrameOutput;
use md_sim::simulation::Simulation;

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
        scene.update_and_render_frame(&mut frame_input, &particles_data);

        FrameOutput::default()
    });
}
