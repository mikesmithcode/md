use glam::DVec3;
use three_d::core::Srgba;
use std::time::Duration;
use std::thread::sleep;
use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder,};

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

    let sim_settings = SimulationSettings{
        dt: 0.0001,
        sim_box_size: [5.0, 5.0, 5.0],
        start: 0,
        stop: 10000,
        sim_filename: String::from("test/test"),
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
            img_filepath: sim_settings.sim_filename.clone(),   
    };

    //Initialise simulation with bunch of particles
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


    let mut simulation = Simulation::new(particles, sim_settings.clone());

    let event_loop = EventLoop::new();
    let mut scene: Scene = Scene::new(scene_settings.clone());
    scene.create_window(&event_loop);
}
