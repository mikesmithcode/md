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
use md::md_sim::force::{add_directional_weight, add_particle_particle_collision, CollisionParams};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::utils::file_io::{SimulationContext, parse_simulation_args, get_latest_file};
use md::md_sim::particle::MoleculeData;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variables {
    pub up: DVec3,
    pub angle: f64,

    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl Variables {
    /// Create a brand-new instance manually without a source file
    pub fn new(up: DVec3, angle: f64) -> Self {
        Self {
            up,
            angle,
            source_path: None,
        }
    }

    /// Scans the directory for the latest variables file, loads it, 
    /// and records its path. Returns `Some(Variables)` or `None`.
    pub fn load_latest(dir_path: &Path) -> Option<Self> {
        let path = get_latest_file(dir_path, "variables", "json")?;
        
        let file = File::open(&path).ok()?;
        let reader = BufReader::new(file);
        
        let mut vars: Variables = serde_json::from_reader(reader).ok()?;
        vars.source_path = Some(path);
        Some(vars)
    }

    /// Saves variables to a 10-digit zero-padded step file if a source path exists.
    /// Returns `Some(())` on success, or `None` if it skipped or failed.
    pub fn save_at_step(&self, step: usize) -> Option<()> {
        let source_path = self.source_path.as_ref()?;

        let parent_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
        let target_path = parent_dir.join(format!("variables_{:010}.json", step));

        let file = File::create(target_path).ok()?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).ok()?;
        
        Some(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForceModel {
    pub collision: CollisionParams,
}

impl ForceModel {
    /// Load force model parameters from a JSON file path (panics cleanly if missing/invalid)
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).expect("Failed to open model config file");
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).expect("Failed to parse model config JSON")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimUpdate {
    pub model: ForceModel,
    #[serde(default)]
    pub variable: Option<Variables>,
}

impl SimUpdate {
    pub fn new(ctx: &SimulationContext) -> Self {
        // Load the mandatory static model config
        let model = ForceModel::load(&ctx.paths.model_config);

        // Automatically scan the output directory for the latest variables_<10-digits>.json file.
        // Returns Some(Variables) if a restart file exists, or None if starting fresh.
        let variable = Variables::load_latest(&ctx.paths.output.join("config"));

        Self { model, variable }
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
        if particles.ptype[i] == 0{
            //Only main particles have granular collisions.
            //println!("possible collide i {}, j {}",particles.ptype[i],particles.ptype[j]); 
            (force, torque)=add_particle_particle_collision(i, j, force, torque,particles, &self.model.collision, settings);
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
