/// Explanation of simulation
/// 
/// Silo consists of a 2D hopper with diagonal walls and a flat bottom. We then drop a square lattice
/// of balls from above into it and watch everything slosh around.


use winit::event_loop::EventLoop;
use glam::{DVec2,DVec3};
use std::collections::HashMap;
use three_d::Srgba;

// Import everything from your md_viz library
use md::md_viz::scene::Scene;

// Imports from simulation library
use md::md_sim::{Forces, Motion, ObjectSpec, ParticleVec, RectSpec, Simulation, SimulationSettings};
use md::md_sim::force::{add_weight, add_granular_collision};
use md::md_sim::motion::{integrate_singleparticle_update, integrate_singleparticle_correct};
use md::md_sim::utils::{filepaths, save_particles, load_latest_particles};
use md::md_sim::particle::MoleculeData;


pub struct SimUpdate;

impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        false
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        true
    }

    fn has_object_forces(&self) -> bool {
        true
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        if particles.ptype[i] == 0{
            force=add_weight(i, force, particles);
        }
        (force, _torque)
    }

    fn update_object_forces(&self, i: usize, force: DVec3, torque: DVec3, particles: &ParticleVec, objects: Option<&[ObjectSpec]>, settings: &SimulationSettings)->(DVec3, DVec3){
        //Assume flat rectangular plane with surface normal in z direction (upwards). Interaction on overlap.
        (force, torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize,mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        (force, torque)
    }

}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], _torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, _molecule_map: &HashMap<usize, MoleculeData>, _time:f64) {
        integrate_singleparticle_update(forces, particles, settings);
    }
    fn correct_motion(&self, forces: &[glam::DVec3], _torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, _molecule_map: &HashMap<usize, MoleculeData>) {
        integrate_singleparticle_correct(forces, particles, settings);
    }
}



pub fn main() {    

    // Construct filepaths
    let [sim_config_path, scene_config_path, _object_path, particle_path, _video_path] = filepaths();
    
    // load settings
    let sim_settings: SimulationSettings = SimulationSettings::new(&sim_config_path).expect("sim settings not loaded correctly"); 

    //------------------------------------------------------------
    // Initialise simulation with bunch of particles from a snapshot file and define simulation parameters with a config file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // -----------------------------------------------------------
    
    let (particles, start_step, time) = load_latest_particles(&particle_path).expect("Failed to return latest snapshot");
    
    
    let size = sim_settings.sim_box_size;
   
    let x=size.x;
    let y=size.y;
    let z=0.005;
    let vertices = [DVec3::new(0.0,0.0, z),DVec3::new(x,0.0, z),DVec3::new(x,y, z),DVec3::new(0.0,y, z)];
    let color = Srgba::RED;

    let rectspec = RectSpec::new(vertices, color);
    let surface = ObjectSpec::Rectangle(rectspec);
    let objects = Some(vec![surface]);



    let mut sim= Simulation::new(particles, objects, SimUpdate, sim_settings.clone(), time);
    //----------------------------------------------------------------
    //  Graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display. Optional video output.
    //  scene.init_headless() for headless video 
    //  Call scene.display() to update window, scene.save_img() to write
    //--------------------------------------------------------------   

    let mut scene: Scene = Scene::from_config(scene_config_path, &sim_settings);   
    let mut event_loop = EventLoop::new(); 
    let _ = scene.view(&event_loop);
    //let _ = scene.start_recording(&video_path, start_step);

    //-------------------------------------------------------------
    // Create simulation
    //
    // Initialise simulation with bunch of particles from a snapshot file. Takes latest snapshot in output
    // copies the config file in input folder to the output folder appending sim index.
    // Simulation::new() creates the simulation
    // sim.update() to advance the simulation by one step
    // file_io::save_snapshot(&snapshot_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot"); for data dump.
    //--------------------------------------------------------------
  
   

    
    
    println!("Simulation started...");
    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_img() to write
    // img to file. simulation.update() to advance the simulation by one step
    //--------------------------------------------------------------
    
    
    // Run simulation loop for num_steps
    for step in start_step..=(start_step+sim.settings.num_steps) {

        sim.update();

        //if step %100 ==0{
        if scene.poll_events(&mut event_loop) {
                break; 
            }
        //}

        // update scene every dump timesteps
        //if step % sim.settings.dump == 0 {
            // exit if window close requested
            
            
            //Handle graphics
            //scene.save_img(&sim.get_particles(), &OUTPUT_PATH, step).expect("Error saving img"); 
            scene.display(sim.get_particles(), sim.get_objects()).expect("Error updating display");
            //let _ = scene.save_frame(&sim.get_particles(), None);

            //save a snapshot of particle positions etc
            save_particles(&particle_path, step, &sim.get_particles(), sim.time).expect("Error saving simulation snapshot");
        //}
        
    }
    scene.close();
    println!("Simulation finished");

}
