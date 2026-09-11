/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.

use winit::event_loop::EventLoop;
use std::path::PathBuf;

use md::md_viz::scene::Scene;
use md::md_viz::scene_settings::SceneSettings;
use md::md_sim::SimulationSettings;
use md::md_sim::utils::{parse_simulation_args, load_particles, load_objects, load_latest_particles, load_latest_objects, SimulationPaths};

pub fn main() {    
    // Construct filepaths
    let ctx = parse_simulation_args();
    let sim_filepaths = ctx.paths;
    let headless = ctx.headless;
    
    // Initialise simulation settings and initial scene state
    let (particles, start_step, _time) = load_latest_particles(&sim_filepaths).expect("Failed to load initial particle snapshot");
    let sim_settings = SimulationSettings::new(&sim_filepaths, start_step).expect("Failed to load simulation settings"); 
    let objects = load_latest_objects(&sim_filepaths).unwrap_or(None);

    // Setup Graphics & Recording
    let event_loop = EventLoop::new(); 
    let scene_settings = SceneSettings::new(&sim_filepaths, &sim_settings); 
    let mut scene = Scene::new(&event_loop, &particles, objects.as_deref(), scene_settings);   
    let _ = scene.start_recording(&sim_filepaths, start_step).expect("Failed to start recording");

    println!("Video started...");

    // Gather, filter, and sort all parquet snapshot files cleanly
    let mut file_paths: Vec<PathBuf> = std::fs::read_dir(&sim_filepaths.particle)
        .expect("Failed to read particle directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "parquet"))
        .collect();
    
    file_paths.sort();

    // Process each frame
    for file_path in file_paths {
        if let Ok((current_particles, _)) = load_particles(&file_path) {
            let step = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<usize>().ok())
                .unwrap_or(0);

            let current_objects = load_objects(&sim_filepaths, step).ok();
            let _ = scene.save_frame(&current_particles, current_objects.as_deref());
        }
    }

    scene.close();
    println!("Video finished");
}