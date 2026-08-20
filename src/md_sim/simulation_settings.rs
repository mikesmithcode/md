//! SimulationSettings
//! 
//! These settings are general rather than particle-specific parameters that affect the running of the simulation.
//! The `SimulationSettings` are loaded predominantly from the `input/<sim_name>.json` file which looks like this:
//! 
//! ```json
//! {
//!   "dt": 1e-5,
//!   "sim_box_size": [0.05, 0.01, 0.05],
//!   "periodic": [true, true, true],
//!   "cutoff": 0.025,
//!   "skin": 0.002,
//!   "start": 0,
//!   "num_steps": 50000,
//!   "dump": 100,
//!   "interaction_ptypes": [[0, 0]],
//!   "model": {
//!     "type": "SolidFriction",
//!     "stiffness": 6650.0,
//!     "damping": 2.97,
//!     "mu": 0.4
//!   }
//! }
//! ```

use glam::DVec3;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::md_sim::particle::{SimulationModel, CollisionParams};

/// Global configuration parameters governing the execution and physical properties of a simulation.
///
/// # Fields
/// * `dt` - The integration time step size.
/// * `sim_box_size` - The $x, y, z$ dimensions of the simulation domain boundary.
/// * `periodic` - Boolean flags enabling or disabling periodic boundary conditions along each axis.
/// * `cutoff` - The interaction cutoff distance within which neighbors are identified by the cell grid / verlet lists.
/// * `skin` - Extra buffer distance added beyond the cutoff; neighbor lists are rebuilt when any particle travels more than `skin / 2`.
/// * `start` - The initial step counter value.
/// * `num_steps` - The total number of steps the simulation will execute before termination.
/// * `dump` - Frequency (in steps) for writing data output files or saving video snapshots.
/// * `interaction_ptypes` - Allowed particle type pairs `[type_a, type_b]` evaluated for forces (non-reciprocal unless explicitly mirrored).
/// * `model` - Specific physical interaction model parameters and configuration. See [`SimulationModel`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationSettings {
    pub dt: f64,
    pub sim_box_size: DVec3, 
    pub periodic: [bool; 3],
    pub cutoff: f64,
    pub skin: f64,
    pub start: usize,
    pub num_steps: usize,
    pub dump: usize,
    pub interaction_ptypes: Vec<[u8; 2]>,
    pub model: SimulationModel,  
}

impl SimulationSettings {
    /// Loads simulation configuration from a JSON file path, providing a formatted error message if unreadable.
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

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            dt: 0.1,
            sim_box_size: DVec3::new(10.0, 0.1, 10.0),
            periodic: [true; 3],
            cutoff: 1.0,
            skin: 0.2,
            start: 0,
            num_steps: 15,
            dump: 1000,
            interaction_ptypes: vec![[0, 0]],
            model: SimulationModel::Solid(CollisionParams {
                stiffness: 1000.0, 
                damping: 50.0,
            }),
        }
    }
}