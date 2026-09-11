//! # 
//!
//! These guides explain the underlying mechanics behind the simulation.
//!
//! ## Table of Contents
//! * [`physics`] — Granular mechanics, caging, and jamming transitions.
//! * [`rendering`] — Camera projection math and instanced mesh structures.
//!
//! ---
//!
//! 
//! [`Orthographic`]: crate::md_viz::camera::create_orthographic_camera
//! [`Perspective`]: crate::md_viz::camera::create_perspective_camera
#![doc = include_str!("../docs/index.md")]

pub mod md_viz;
pub mod md_sim;


/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.

use crate::md_sim::{Forces, Motion, Simulation, SimulationSettings};
use crate::md_sim::utils::{parse_simulation_args, save_particles, load_latest_particles, load_latest_objects};
use crate::md_viz::init_scene;


pub fn run_simulation<U: Forces + Motion + Sync>(sim_update: U) {
    let ctx = parse_simulation_args();
    let sim_filepaths = ctx.paths;
    let headless = ctx.headless;
    let record = ctx.record_video;
    
    let (particles, start_step, time) = load_latest_particles(&sim_filepaths).expect("Failed snapshot");
    let sim_settings = SimulationSettings::new(&sim_filepaths, start_step).expect("settings failed"); 
    let objects = load_latest_objects(&sim_filepaths).expect("objects failed");

    let mut sim = Simulation::new(particles, objects, sim_update, sim_settings.clone(), time);

    let mut scene_tuple = init_scene(
        headless,
        record,
        &sim_filepaths,
        &sim_settings,
        sim.get_particles(),
        sim.get_objects(),
        start_step,
    );

    println!("Simulation started (Headless: {}, Record Video: {})...", headless, record);

    for step in start_step..=(start_step + sim.settings.num_steps) {
        sim.update();

        if step % sim.settings.dump == 0 {
            println!("step {}", step);

            if let Some((event_loop, scene)) = scene_tuple.as_mut() {
                if !headless {
                    if scene.poll_events(event_loop) {
                        break;
                    }
                    scene.display(sim.get_particles(), sim.get_objects()).expect("Error displaying");
                }

                if record {
                    let _ = scene.save_frame(sim.get_particles(), None);
                }
            }

            save_particles(&sim_filepaths, step, sim.get_particles(), sim.time).expect("Error saving particles");
        }
    }

    if let Some((_, mut scene)) = scene_tuple {
        scene.close();
    }

    println!("Simulation finished");
}
