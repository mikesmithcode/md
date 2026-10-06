use glam::DVec3;
use std::collections::HashMap;

use crate::md_sim::force::CellGrid;
use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{ParticleVec, MoleculeData};



/// Constructs a lookup map containing metadata and inertial properties for a single test molecule.
pub fn setup_single_molecule_data(particles: &ParticleVec) -> HashMap<usize, MoleculeData> {
    let mut map = HashMap::new();
    let pids = vec![0, 1];
    let mol_data = MoleculeData::new(pids, particles);
    map.insert(0, mol_data);
    map
}

/// Initializes a default test simulation cell grid and corresponding simulation settings.
pub fn create_grid_and_settings() -> (CellGrid, SimulationSettings) {
    let particle_count = 6;
    let settings = SimulationSettings {
        skin: 0.2,
        sim_box_size: DVec3::splat(9.0),
        periodic: [true; 3],
        interaction_ptypes: vec![(0, 1, 2.8)],
        ..Default::default()
    };

    let grid = CellGrid::new(particle_count, &settings);
    (grid, settings)
}

pub fn assert_dvec3_near(actual: DVec3, expected: DVec3, eps: f64) {
    let diff = (actual - expected).length();
    assert!(
        diff < eps,
        "Expected {:?}, got {:?} (diff: {})",
        expected, actual, diff
    );
}
