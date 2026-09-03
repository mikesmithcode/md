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
use md::md_sim::utils::{filepaths, save_particles, load_latest_particles, load_latest_objects, SimulationPaths};
use md::md_sim::particle::MoleculeData;



pub struct SimUpdate;

impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
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
        // Only the main particle has weight
        if particles.ptype[i] == 0{
        force = add_weight(i, force, particles);
        }
        (force, _torque)
    }

    fn update_object_forces(&self, i: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec, objects: &ObjectSpec, settings: &SimulationSettings)->(DVec3, DVec3){
        //Only main particle collides with the surface
        if particles.ptype[i] == 0{
           (force,torque) = add_particle_object_collision(i, particles, objects, force, torque, settings);
        }
        (force, torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        // guaranteed that i and j will be same ptype due to verlet list specs
        if particles.ptype[i] == 0{
            //Only main particles have granular collisions. 
            (force, torque)=add_particle_particle_collision(i, j, particles, force, torque, settings);
        }
        else{
            // ptype == 1 is the charge.
            force = add_coulomb(i, j, particles, force, settings);
        }

    
        (force, torque)
    }

}

impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>, _time:f64) {
        integrate_rigid_bodies(forces,torques, particles, molecule_map, settings);
    }

    fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>) {
        integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings);
    }

    fn update_objects(&self, object: &mut ObjectSpec, settings: &SimulationSettings, time: f64){
        match object {
            ObjectSpec::Rectangle(rect) => {
                let ang_freq = 1000.0;
                let velocity = DVec3::new(0.0,0.0,0.15)*f64::sin(ang_freq*time);
                rect.step(velocity, DVec3::ZERO, settings.dt);
            },
            _ => {}   
        }
    }

}




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
    let mut sim= Simulation::new(particles, objects, SimUpdate, sim_settings.clone(), time);

    //----------------------------------------------------------------
    //  Setup Graphics
    //
    //  event_loop and scene.init_window(&event_loop) for live display. Optional video output.
    //  scene.init_headless() for headless video 
    //  Call scene.display() to update window, scene.save_img() to write
    //--------------------------------------------------------------   
    let mut event_loop = EventLoop::new(); 
    let scene_settings: SceneSettings = SceneSettings::new(&sim_filepaths, &sim_settings); 
    let mut scene: Scene = Scene::new(&event_loop, sim.get_particles(), sim.get_objects(), scene_settings.clone());   
    //let _ = scene.start_recording(&sim_paths, start_step);


    
    //--------------------------------------------------------------
    // Start simulation loop
    //
    // Call scene.display() to update window, scene.save_img() to write
    // img to file. simulation.update() to advance the simulation by one step
    //--------------------------------------------------------------
    println!("Simulation started...");
    
    // Run simulation loop for num_steps
    for step in start_step..= (start_step+sim.settings.num_steps){

        sim.update();
        if step % 100 == 0 && scene.poll_events(&mut event_loop) {
            break;
        }

        // update scene every dump timesteps
        if step % sim.settings.dump == 0 {
            // exit if window close requested
            
            
            //Handle graphics
            //scene.save_img(&sim.get_particles(), &OUTPUT_PATH, step).expect("Error saving img");
            
            scene.display(sim.get_particles(), sim.get_objects()).expect("Error updating display");
            //let _ = scene.save_frame(&sim.get_particles(), None);

            //save a snapshot of particle positions etc
            save_particles(&sim_filepaths, step, sim.get_particles(), sim.time).expect("Error saving particles snapshot");
            //save_objects(&sim_filepaths, step, sim.get_objects(), sim.time).expect("Error saving objects snapshot");

        }
        
    }
    scene.close();
    println!("Simulation finished");

}
