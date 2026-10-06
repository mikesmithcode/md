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
use md::md_sim::force::{add_directional_weight, check_particle_contact, CollisionParams};
use md::md_sim::force::{normal_linear, friction_viscous_tangential, ContactManager, normal_hertzian, friction_cundall_strack};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::file_io::{SimulationContext, parse_simulation_args, get_latest_file};
use md::md_sim::particle::MoleculeData;


    

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForceModel {
    pub collision: CollisionParams,
    pub gravity: Gravity,
    #[serde(skip)]
    pub source_path: PathBuf,
}

impl ForceModel {
    /// Scans the directory for the latest variables file, loads it, 
    /// and records its path.
    pub fn load_latest(dir_path: &Path) -> Self {
        let path = get_latest_file(dir_path, "model", "json")?;
        
        let file = File::open(&path).ok()?;
        let reader = BufReader::new(file);
        
        let mut model: ForceModel = serde_json::from_reader(reader).ok()?;
        model.source_path = path;
        
        model
    }

    /// Saves variables to a 10-digit zero-padded step file if a source path exists.
    pub fn save_at_step(&self, step: usize) {
        let source_path = self.source_path.as_ref()?;

        let parent_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
        let target_path = parent_dir.join(format!("model_{:010}.json", step));

        let file = File::create(target_path).ok()?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).ok()?;
    }
}


#[derive(Clone, Debug, Serialize)]
pub struct SimUpdate {
    pub model: ForceModel,
    #[serde(skip)]
    pub contact_manager: ContactManager,
}

impl SimUpdate {
    pub fn new(ctx: &SimulationContext) -> Self {
        // Load the mandatory static model config
        let model = ForceModel::load_latest(&&ctx.paths.output.join("config"));

        let contact_manager = ContactManager::new();

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

    fn cleanup_contacts(&self) {
        self.contact_manager.remove_old_contacts();
    }


    //Forces which apply to every particle individually
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        // Only the dynamic particle has weight
        if particles.ptype[i] == 0{
            let var = self.variable.as_ref().expect("Variables are required for this simulation");
            force = add_directional_weight(i, force, particles, var.up);
        }
        (force, _torque)
    }

    // forces that operate between pairs of particles
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        
        //Only main particles have granular collisions.
        if particles.ptype[i] == 0{
            // Calculate geometry and params of contact
            if let Some(contact)=check_particle_contact(i, j, particles, &self.model.collision, settings){
                //Normal and tangential forces
                //at the end of every timestep we set all ContactState.is_active to false
                self.contact_manager.get_or_create(i,j);
                let fn_vec=normal_linear(&contact);
                //let ft_vec = friction_viscous_tangential(fn_vec.length(), &contact);
                let ft_vec = friction_cundall_strack(fn_vec, &contact, self.contact_state, settings.dt);

                //Add to accumulators
                force += fn_vec;
                force += ft_vec;
                torque += contact.r_contact.cross(ft_vec);


            }
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
}


// In SimUpdate (where Variables is fully in scope):
impl Interactivity for SimUpdate {
    fn save_variables(&self, step: usize) {
        if let Some(ref vars) = self.variable {
            let _ = vars.save_at_step(step);
        }
    }
}






pub fn main() {    
    let ctx = parse_simulation_args();
    md::run_simulation(SimUpdate::new(&ctx), ctx);
}
