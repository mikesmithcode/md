/// benchmarksetup.rs
/// 
/// Cube of particles are dropped under gravity onto a grid of fixed particles. This is used as a setup script for benchmark.rs


use glam::DVec3;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::io::{BufReader, BufWriter};
use std::fs::File;


// Imports from simulation library
use md::md_sim::{Forces, Interactivity, Motion, ParticleVec, ObjectSpec, SimulationSettings};
use md::md_sim::force::{FrictionCundallStrack, Gravity, NormalForce, add_gravity, check_particle_contact, check_surface_contact};
use md::md_sim::force::{normal_linear, friction_cundall_strack, ContactManager};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::motion::Vibration;
use md::md_sim::utils::file_io::{SimulationContext, parse_simulation_args, get_latest_file};
use md::md_sim::particle::MoleculeData;


    

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub normal_force: NormalForce,
    pub friction: FrictionCundallStrack,
    pub gravity: Gravity,
    pub vibration: Vibration,
}

impl Model {
    /// Scans the directory for the latest variables file, loads it, 
    /// and records its path.
    pub fn load_latest(ctx: &SimulationContext) -> Self {
        let config_dir = ctx.paths.output.join("config");
        let latest_file_path = get_latest_file(&config_dir, "model", "json").expect("Error getting latest file");
        
        let latest_file = File::open(&latest_file_path).expect("Error opening file");
        let reader = BufReader::new(latest_file);
        let model: Model = serde_json::from_reader(reader).expect("Error creating struct from file");
        
        model
    }

    /// Saves model to a 10-digit zero-padded step file if a source path exists.
    pub fn save_at_step(&self, ctx: &SimulationContext, step: usize) {
        let config_dir = ctx.paths.output.join("config");
        let target_path = config_dir.join(format!("model_{:010}.json", step));
        let file = File::create(target_path).expect("Error creating file");
        let writer = BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, self);
    }
}


#[derive(Clone, Debug, Serialize)]
pub struct SimUpdate {
    pub model: Model,
    #[serde(skip)]
    pub contact_manager: ContactManager,
    pub ctx: SimulationContext,
}

impl SimUpdate {
    /// ==============================================================================
    /// new
    /// ==============================================================================
    /// Initializes a new simulation update context by loading the latest force model 
    /// configuration and restoring prior contact states if available.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A reference to the global [`SimulationContext`] managing paths and runtime flags.
    ///
    /// # Returns
    ///
    /// * `Self` - The initialized `SimUpdate` instance containing models and contact data.
    pub fn new(ctx: SimulationContext) -> Self {
        // Load the latest model config from the output config directory
        let model = Model::load_latest(&ctx);

        // Attempt to load the latest saved contact state; fallback to a fresh manager if none exists
        let contact_manager = ContactManager::load_latest(&ctx)
            .unwrap_or_else(|| {
                println!("New ContactManager Created");
                ContactManager::new()
                
            });

        Self { model, contact_manager, ctx}
    }
}



impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        true
    }
    //Default implementation is false set to true if using
    fn has_object_forces(&self) -> bool {
        true
    }

    
    //Only needed if you are contact tracking
    fn cleanup_contacts(&mut self) {
        
        self.contact_manager.remove_old_contacts();
    }


    //Forces which apply to every particle individually, e.g. gravity
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the dynamic particle has weight
        if particles.ptype[i] == 0{
            force = add_gravity(i, force, particles, &self.model.gravity);
        }
        (force, _torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        //Only main particles have granular collisions.
        if particles.ptype[i] == 0{
            // Calculate geometry and params of contact
            if let Some(contact)=check_particle_contact(i, j, particles, settings){
                //Normal and tangential forces
                //at the end of every timestep we set all ContactState.is_active to false
                self.contact_manager.check_or_add((i,j), DVec3::ZERO);
                let (fn_mag, fn_vec)=normal_linear(&contact, &self.model.normal_force);
                //let ft_vec = friction_viscous_damping(fn_mag, &contact, &self.model.friction);
                let ft_vec = friction_cundall_strack(i,j,fn_mag, &contact, &self.contact_manager, &self.model.friction, settings.dt);

                //Add to accumulators
                force += fn_vec;
                force += ft_vec;
                torque += contact.r_contact.cross(ft_vec);


            }
        }
    
        (force, torque)
    }

    fn update_object_forces(&self,i: usize,force: DVec3,torque: DVec3,particles: &ParticleVec,object_spec: &ObjectSpec,settings: &SimulationSettings) -> (DVec3, DVec3) {
        
        if let Some(contact) = check_surface_contact(i, particles, object_spec, settings){
            

        }
        
        
        
        (force, torque)
    }
}


impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>, _time:f64) {
        integrate_rigid_bodies(forces,torques, particles, molecule_map, settings);
    }

    // Optional depending on integration scheme.
    fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &HashMap<usize, MoleculeData>) {
        integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings);
    }

    fn update_objects(&mut self, object: &mut ObjectSpec, _particles: &mut ParticleVec,settings: &SimulationSettings, time: f64){
        match object {
            ObjectSpec::Rectangle(rect) => {
                let ang_freq = self.model.vibration.ang_freq;
                let velocity = self.model.vibration.velocity*f64::sin(ang_freq*time);
                rect.step(velocity, DVec3::ZERO, settings.dt);
            },
            _ => {}   
        }

    }
}


// In SimUpdate :
impl Interactivity for SimUpdate {

    fn ctx(&self)-> &SimulationContext{
        &self.ctx
    }

    //particles and objects are saved but if you need model params or contacts saved add here.
    fn save_on_exit(&self, step:usize)-> Option<()>{
        self.contact_manager.save_at_step(self.ctx(), step);
        self.model.save_at_step(self.ctx(), step);
        Some(())
    }

    //For implementing actions when key is pressed.
    //fn handle_key(&mut self, key: md::md_viz::actions::UserAction, _particles: &mut ParticleVec, _objects: &mut Option<Vec<md::md_sim::ObjectSpec>>, step: usize){}


    
}






pub fn main() {    
    let ctx = parse_simulation_args();
    let sim_update = SimUpdate::new(ctx);
    md::run_simulation(sim_update);
}
