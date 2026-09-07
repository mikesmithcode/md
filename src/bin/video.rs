/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use winit::event_loop::EventLoop;
use glam::DVec3;
use std::collections::HashMap;

// Import everything from your md_viz library
use md::md_viz::scene::Scene;
use md::md_viz::scene_settings::SceneSettings;

// Imports from simulation library
use md::md_sim::{Forces, Motion, ObjectSpec, ParticleVec, Simulation, SimulationSettings};
use md::md_sim::force::{add_coulomb, add_particle_object_collision, add_particle_particle_collision, add_weight};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::{filepaths, save_particles, save_objects, load_latest_particles, load_latest_objects, SimulationPaths};
use md::md_sim::particle::MoleculeData;




pub fn main() {    

    // Construct filepaths
    let sim_filepaths: SimulationPaths = filepaths();
    
    //------------------------------------------------------------
    // Initialise simulation with bunch of particles from a snapshot file and define simulation parameters with a config file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // -----------------------------------------------------------
    let (particles, start_step, time) = load_latest_particles(&sim_filepaths).expect("Failed to return latest particle snapshot");
    

    // load settings
    let sim_settings: SimulationSettings = SimulationSettings::new(&sim_filepaths, start_step).expect("sim settings not loaded correctly"); 
    
    //--------------------------------------------------------------
    //Load surface
    //--------------------------------------------------------------
    let objects = load_latest_objects(&sim_filepaths).expect("Failed to return latest object snapshot");

    //-------------------------------------------------------------
    // Create simulation
    //
    // Initialise simulation with bunch of particles from a snapshot file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // Simulation::new() creates the simulation
    // sim.update() to advance the simulation by one step
    // If you have no objects supply None.
    // file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot"); for data dump.
    //--------------------------------------------------------------  
    //let mut sim= Simulation::new(particles, objects, SimUpdate, sim_settings.clone(), time);

    //----------------------------------------------------------------
    //  Setup Graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display. Optional video output.
    //  scene.init_headless() for headless video 
    //  Call scene.display() to update window, scene.save_img() to write
    //--------------------------------------------------------------   
    let mut event_loop = EventLoop::new(); 
    let scene_settings: SceneSettings = SceneSettings::new(&sim_filepaths, &sim_settings); 
    let mut scene: Scene = Scene::new(&event_loop, &particles, objects.as_deref(), scene_settings.clone());   
    let _ = scene.start_recording(&sim_filepaths, start_step); // sim_filepaths will handle where the video gets stored.


    
    //--------------------------------------------------------------
    // Make Video
    //-------------------------------------------------------------
    println!("Video started...");

    //Find all the files in the particles and objects folder
    // #[derive(Default)]
/// Encapsulates all major file paths required for running and saving a simulation.
//pub struct SimulationPaths {
//    pub output: PathBuf,
//    pub sim_config: PathBuf,
//    pub scene_config: PathBuf,
//    pub object: PathBuf,
//    pub particle: PathBuf,
//    pub video: PathBuf,
//}

    // We have these two functions which when given a filepath will return the objects and particles needed for the scene to render an image
    // pub fn load_objects(sim_paths: &SimulationPaths, step: usize) -> Result<Vec<ObjectSpec>, Box<dyn std::error::Error>>
    // pub fn load_particles(file_path: &Path) -> Result<(ParticleVec, f64), Box<dyn std::error::Error>> 
    // the SimulationPaths object and particle provide the paths to the folders where the particles_0000000000.parquet and objects_0000000000.parquet etc are stored.
    // need to sort and loop over all files in each folder. Read them in. Pass to .save_frame
    
                
    let _ = scene.save_frame(&particles, objects.as_deref());

            
    scene.close();
    println!("Video finished");

}
