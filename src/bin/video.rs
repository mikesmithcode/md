use md::md_sim::file_io;
use md::md_viz::scene::Scene;
use md::md_sim::simulation::SimulationSettings;
use std::path::{Path, PathBuf};
use glob::glob;
use winit::event_loop::EventLoop;


fn main(){

    let sim_name = std::env::args().nth(1).expect("Must supply a simulation name");

    let [sim_config_path, scene_config_path, _snapshot_path, video_path] = file_io::filepaths(&sim_name);
    
    let sim_settings: SimulationSettings = SimulationSettings::new(&sim_config_path).expect("sim settings not loaded correctly"); 
    let mut scene: Scene = Scene::from_config(scene_config_path, &sim_settings);
    let event_loop = EventLoop::new();   
    scene.background(&event_loop).expect("Error creating headless window");
    scene.start_recording(&video_path,0).expect("Error initialising video");
    
    let pattern = Path::new("output").join(sim_name).join("snapshots").join("snapshot_*.parquet");

    if let Some(pattern_str) = pattern.to_str() {
        let mut entries: Vec<PathBuf> = glob(pattern_str).expect("Failed to read glob pattern").filter_map(Result::ok).collect();
        entries.sort(); 
        
        for entry in entries{
            if let Ok((particles, _)) =file_io::load_snapshot(&entry){
                scene.save_frame(&particles).expect("Error saving frame");            }
            
        }
    }
}
