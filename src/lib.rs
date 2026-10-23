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

use crate::md_sim::{Forces, Motion, InputOutput, Simulation, SimulationSettings};
use crate::md_sim::utils::{save_particles, save_objects, load_latest_particles, load_latest_objects};
use crate::md_viz::{init_scene, actions::UserAction};

#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        // cfg!(debug_assertions) is evaluated at compile time. 
        // In release/max-perf builds, the compiler completely strips this out.
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

#[cfg(feature = "profiling")]
pub mod profiler {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    use std::sync::LazyLock;
    use dashmap::DashMap;

    // Global thread-safe registry mapping scope names to (total_nanos, call_count)
    pub static PROFILER_REGISTRY: LazyLock<DashMap<&'static str, (AtomicU64, AtomicU64)>> = 
        LazyLock::new(DashMap::new);

    pub struct ProfileGuard {
        name: &'static str,
        start: Instant,
    }

    impl ProfileGuard {
        #[inline(always)]
        pub fn new(name: &'static str) -> Self {
            Self {
                name,
                start: Instant::now(),
            }
        }
    }

    impl Drop for ProfileGuard {
        fn drop(&mut self) {
            let elapsed_ns = self.start.elapsed().as_nanos() as u64;
            let entry = PROFILER_REGISTRY
                .entry(self.name)
                .or_insert_with(|| (AtomicU64::new(0), AtomicU64::new(0)));
            
            entry.0.fetch_add(elapsed_ns, Ordering::Relaxed);
            entry.1.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn print_summary() {
        println!("\n==========================================================================");
        println!("{:45} | {:<12} | {:<10} | {:<12}", "Scope Name", "Total Time", "Calls", "Avg / Call");
        println!("--------------------------------------------------------------------------");

        // Sort items by total time descending for easy reading
        let mut metrics: Vec<(&'static str, Duration, u64)> = PROFILER_REGISTRY
            .iter()
            .map(|item| {
                let name = *item.key();
                let total_ns = item.value().0.load(Ordering::Relaxed);
                let count = item.value().1.load(Ordering::Relaxed);
                (name, Duration::from_nanos(total_ns), count)
            })
            .collect();

        metrics.sort_by(|a, b| b.1.cmp(&a.1));

        for (name, total, count) in metrics {
            let avg = if count > 0 { total / count as u32 } else { Duration::ZERO };
            println!("{:45} | {:<12?} | {:<10} | {:<12?}", name, total, count, avg);
        }
        println!("==========================================================================\n");
    }
}

// Macro when profiling is DISABLED (expands to absolute nothingness)
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {};
}

#[cfg(not(feature = "profiling"))]
pub fn print_profile_summary() {}

// Macro when profiling is ENABLED
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _profiler_guard = $crate::profiler::ProfileGuard::new($name);
    };
}

#[cfg(feature = "profiling")]
pub use profiler::print_summary as print_profile_summary;


pub fn run_simulation<U: Forces + Motion + InputOutput + Sync>(sim_update: U) {
    let sim_filepaths = sim_update.ctx().paths.clone();
    let output_settings = sim_update.ctx().output_settings.clone();
    
    let (particles, start_step, time) = load_latest_particles(&sim_filepaths).expect("Failed snapshot");
    let objects = load_latest_objects(&sim_filepaths).expect("objects failed");
    
    let sim_settings = SimulationSettings::new(&sim_filepaths, start_step).expect("settings failed"); 

    let mut sim = Simulation::new(particles, objects, sim_update, sim_settings.clone(), time);

    // init_scene now returns Option<Scene> directly because Scene owns its EventLoop
    let mut scene_opt = init_scene(
        output_settings.headless,
        output_settings.record_video,
        &sim_filepaths,
        &sim_settings,
        sim.get_particles(),
        sim.get_objects(),
        start_step,
    );

    println!("Simulation started (Headless: {}, Record Video: {})...", output_settings.headless, output_settings.record_video);

    let mut final_step = start_step;

    for step in start_step..=(start_step + sim.settings.num_steps) {
        {
            profile_scope!("update loop");
            sim.update(step);
        }
        final_step = step;

        if let Some(ref mut scene) = scene_opt {
            // Determine if this step requires the scene to touch the particles
            let is_display_step = step % scene.scene_settings.display_steps == 0;
            let is_recording = output_settings.record_video; // (plus any frequency check if needed)

            // Only poll events and handle window interaction if we are actively displaying live
            if !output_settings.headless && is_display_step {
                profile_scope!("polling window");
                let (close_requested, action) = scene.poll_events();
                if close_requested {
                    break;
                }

                match action {
                    UserAction::None => {}
                    other_action => {
                        sim.handle_key(other_action);
                    }
                }
            }

            // If we are either displaying *or* recording this step, update the visual buffers!
            if (!output_settings.headless && is_display_step) || is_recording {
                profile_scope!("updating display_pos");
                sim.update_display_pos(&scene.scene_settings);
            }

            // Render to screen if it's time
            if !output_settings.headless && is_display_step {
                profile_scope!("rendering to screen");
                scene.display(sim.get_particles(), sim.get_objects()).expect("Error displaying");
            }

            // Save frame for video if enabled
            if is_recording {
                profile_scope!("recording video");
                let _ = scene.save_frame(sim.get_particles(), sim.get_objects());
            }
        }

        // Write particles and / or objects to parquet files
        if step % sim.settings.dump == 0 {
            if output_settings.save_particles {
                save_particles( sim.get_particles(), &sim_filepaths, step,sim.time).expect("Error saving particles");
            }

            if output_settings.save_objects {
                save_objects(sim.get_objects(),&sim_filepaths, step,  sim.time).expect("Error saving objects");
            }
        }
    }

    // Guaranteed final backup save on exit/completion
    println!("Saving final state at step {}...", final_step);
    save_particles(sim.get_particles(),&sim_filepaths, final_step,  sim.time).expect("Error saving final particles");
    save_objects(sim.get_objects(),&sim_filepaths, final_step, sim.time).expect("Error saving final objects");
    sim.sim_update.save_on_exit(final_step).expect("Error saving contacts.json");

    if let Some(mut scene) = scene_opt {
        scene.close();
    }

    println!("Simulation finished");
    
    crate::print_profile_summary();
}


