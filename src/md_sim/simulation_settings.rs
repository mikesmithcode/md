//!SimulationSettings
//! 
//! These settings are general rather than particle specific parameters that affect the running of the simulation
//! The SimulationSettings are loaded predominantly from the input/<sim_name>.json file which looks like this:
//! 
//! ```json
//!   {
//!  "dt": 1e-5,
//!  "sim_box_size": [0.05, 0.01, 0.05],
//!  "periodic": [true,true,true],
//!  "cutoff": 0.025,
//!  "skin": 0.002,
//!  "start": 0,
//!  "num_steps": 50000,
//!  "dump": 100,
//!  "interaction_ptypes": [[0,0]],
//!  "model": {
//!    "type": "SolidFriction",
//!    "stiffness": 6650.0,
//!    "damping": 2.97,
//!    "mu": 0.4
//!  }
//! }
//!  ```
//! 



use glam::DVec3;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::md_sim::particle::{SimulationModel, CollisionParams};


///---------------------------------------------------------
/// Definitions of SimulationSettings
/// 
/// dt - timestep of the simulation
/// sim_box_size - x,y,z dimensions of the simulation box
/// periodic - can turn periodic on - true or off false in each dimension.
/// cutoff - range of force or distance within which neighbours are defined by the cell grid / verlet in [`crate::md_sim::neighbours::CellGrid`]
/// skin -  This is the distance beyond the cutoff in which particles are added to a particles verlet list. When any particle travels skin/2 the grid and verlet list are rebuilt.
/// start - initial step number.
/// num_steps - How many steps the simulation will advance before stopping
/// dump - Can be used to control how many steps occur before writing to a file or saving an image to the video. But must be used manually in the main loop
/// interactive_ptypes - A Vec of 2 x i32 arrays where each number represents an interaction between particles of ptype [1,2] means ptype 1 will experience a force from ptype 2. This does 
/// not imply reciprocity. If you want that specify [1,2],[2,1]. This is used to optimise the particle neighbour lists to speed everything up.
/// model - used to try and get additional parameters for different types of simulation into the simulation. See [`SimulationModel`].
/// 
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings{
    pub dt: f64,
    pub sim_box_size: DVec3, 
    pub periodic: [bool; 3],
    pub cutoff: f64,
    pub skin: f64,
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    pub interaction_ptypes: Vec<[u8;2]>,
    pub model: SimulationModel,  
}

impl SimulationSettings {
    /// Loads sim config from file and builds the active mask
    pub fn new(path: &Path) -> Result<SimulationSettings, Box<dyn std::error::Error>> {
        let file = File::open(path).map_err(|e| {
            format!(
                "\n==========================================\n\
                Error: Couldn't find config at {}\n\
                Details: {}\n\
                ==========================================\n", 
                path.display(), e
            )
        })?;
        
        let reader = BufReader::new(file);
        let sim_settings: SimulationSettings = serde_json::from_reader(reader)?;

        Ok(sim_settings)
    }
}

/// Largely used for testing
impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            dt: 0.1,
            sim_box_size: DVec3::new(10.0, 0.1, 10.0),
            periodic: [true;3],
            cutoff: 1.0,
            skin:0.2,
            start: 0,
            num_steps: 15,
            dump: 1000,
            interaction_ptypes: vec![[0,0]],
            //head_ptypes: vec![],
            model: SimulationModel::Solid(CollisionParams{
                stiffness: 1000.0, 
                damping: 50.0}),
        }

    }
}
