/// Explanation of simulation
/// 
/// This script is used to create the initial condition for the benchmark


use glam::DVec3;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::io::{BufReader, BufWriter};
use std::fs::File;
use std::path::{Path, PathBuf};

// Import everything from your md_viz library


// Imports from simulation library
use md::md_sim::{Forces, Interactivity, Motion, ParticleVec, SimulationSettings};
use md::md_sim::force::{add_gravity, Gravity, check_particle_contact, };
use md::md_sim::force::{normal_linear, friction_viscous_damping, NormalForce, FrictionViscous, friction_cundall_strack, ContactManager};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::file_io::{SimulationContext, parse_simulation_args, get_latest_file};
use md::md_sim::particle::MoleculeData;


    

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForceModel {
    pub normal_force: NormalForce,
    pub friction: FrictionViscous,
    pub gravity: Gravity,
    #[serde(skip)]
    pub source_path: PathBuf,
}

impl ForceModel {
    /// Scans the directory for the latest variables file, loads it, 
    /// and records its path.
    pub fn load_latest(dir_path: &Path) -> Self {
        let path = get_latest_file(dir_path, "model", "json").expect("Error getting latest file");
        
        let file = File::open(&path).expect("Error opening file");
        let reader = BufReader::new(file);
        
        let mut model: ForceModel = serde_json::from_reader(reader).expect("Error creating struct from file");
        model.source_path = path;
        
        model
    }

    /// Saves variables to a 10-digit zero-padded step file if a source path exists.
    pub fn save_at_step(&self, step: usize) {
        let source_path = &self.source_path;

        let parent_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
        let target_path = parent_dir.join(format!("model_{:010}.json", step));

        let file = File::create(target_path).expect("Error creating file");
        let writer = BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, self);
    }
}


#[derive(Clone, Debug, Serialize)]
pub struct SimUpdate {
    pub model: ForceModel,
    #[serde(skip)]
    pub contact_manager: ContactManager,
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
    pub fn new(ctx: &SimulationContext) -> Self {
        // Load the mandatory static model config from the output config directory
        let config_dir = ctx.paths.output.join("config");
        let model = ForceModel::load_latest(&config_dir);

        // Attempt to load the latest saved contact state; fallback to a fresh manager if none exists
        let contact_manager = ContactManager::load_latest(&config_dir)
            .unwrap_or_else(|| {
                ContactManager::new(config_dir)
            });

        Self { model, contact_manager }
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

    fn has_object_forces(&self) -> bool {
        false
    }

    //Only needed if you are contact tracking
    fn cleanup_contacts(&mut self) {
        self.contact_manager.remove_old_contacts();
    }


    //Forces which apply to every particle individually
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
                let ft_vec = friction_viscous_damping(fn_mag, &contact, &self.model.friction);
                //let ft_vec = friction_cundall_strack(fn_vec, &contact, self.contact_state, settings.dt);

                //Add to accumulators
                force += fn_vec;
                force += ft_vec;
                torque += contact.r_contact.cross(ft_vec);


            }
        }
    
        (force, torque)
    }

    
    fn save_on_exit(&self, step:usize)-> Option<()>{
        self.contact_manager.save_at_step(step);
        self.save_changes_model(step);
        Some(())
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
}


// In SimUpdate :
impl Interactivity for SimUpdate {
    
    fn save_changes_model(&self, step: usize) { 
        self.model.save_at_step(step);
    }

    //For implementing actions when key is pressed.
    //fn handle_key(&mut self, key: md::md_viz::actions::UserAction, _particles: &mut ParticleVec, _objects: &mut Option<Vec<md::md_sim::ObjectSpec>>, step: usize){}
    
}






pub fn main() {    
    let ctx = parse_simulation_args();
    md::run_simulation(SimUpdate::new(&ctx), ctx);
}
