

use super::*;
use tempfile::tempdir;
use glam::{DVec3, DQuat};
use three_d::Srgba;
use std::path::Path;

use crate::md_sim::{Particle, ParticleVec};

const NULL_ID: usize = usize::MAX;


// -----------------------------------------------------------------
// Test utility functions
// -----------------------------------------------------------------

#[test]
fn test_check_delta() {
    let sim_box_size = DVec3::new(10.0, 10.0, 10.0);
    let periodic = [true;3];
    // Case 1: X is far apart (0.9L), should wrap to a small negative distance (-0.1L)
    // Example: Particle A at 0.5, Particle B at 9.5. Delta = 9.0
    let mut delta_x = DVec3::new(9.0, 0.0, 0.0);
    check_delta(&mut delta_x, sim_box_size, periodic);
    assert!((delta_x.x + 1.0).abs() < 1e-6); // 9.0 - 10.0 = -1.0

    // Case 2: Y is negative and far apart, should wrap to a small positive distance
    // Example: Particle A at 9.5, Particle B at 0.5. Delta = -9.0
    let mut delta_y = DVec3::new(0.0, -9.0, 0.0);
    check_delta(&mut delta_y, sim_box_size, periodic);
    assert!((delta_y.y - 1.0).abs() < 1e-6); // -9.0 + 10.0 = 1.0

    // Case 3: Z is already the shortest path, should remain unchanged
    let mut delta_z = DVec3::new(0.0, 0.0, 2.0);
    check_delta(&mut delta_z, sim_box_size, periodic);
    assert!((delta_z.z - 2.0).abs() < 1e-6);
}

// -----------------------------------------------------------------
// Test file_io functions
// -----------------------------------------------------------------

#[test]
fn test_filepath()-> Result<(), Box<dyn std::error::Error>>{
    let [sim_config_path, _scene_config_path, _snapshot_path, _video_path] = filepaths("test.rs");

    assert_eq!(sim_config_path, Path::new("input").join("test.json"));

    Ok(())
}

#[test]
fn test_save_and_load_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    // Setup temporary workspace
    let dir = tempdir()?;
    let dir_path = dir.path();
    
    // Create dummy particle data
    let mut particles = ParticleVec::new();
    particles.push(
        Particle {
            id: 1,
            molecule_id: NULL_ID,
            ptype: 0,
            position: DVec3::new(1.0, 2.0, 3.0),
            rel_pos: DVec3::ZERO,
            velocity: DVec3::new(0.1, 0.2, 0.3),
            orientation: DQuat::IDENTITY,
            omega: DVec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            mass: 1.0,
            charge: 0.0,
            color: Srgba::new(255, 0, 0, 255),
            ref_pos: DVec3::ZERO,
        });
    let step = 42;
    let time = 0.5;

    // Test saving
    save_snapshot(dir_path, step, &particles, time)?;

    // Test loading specific file
    let file_name = format!("snapshot_{:010}.parquet", step);
    let file_path = dir_path.join(file_name);
    let (loaded_particles, loaded_time) = load_snapshot(&file_path)?;

    // Checks
    assert_eq!(loaded_particles.len(), 1);
    assert_eq!(loaded_particles.id[0], 1);
    assert_eq!(loaded_time, 0.5);
    assert!((loaded_particles.position[0].x - 1.0).abs() < f64::EPSILON);
    
    Ok(())
}

#[test]
fn test_load_latest_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let dir_path = dir.path();
    
    // Save two snapshots with different steps
    let mut particles = ParticleVec::new(); 

    particles.push(
        Particle {
            id: 1,
            molecule_id: NULL_ID,
            ptype: 0,
            position: DVec3::new(1.0, 2.0, 3.0),
            rel_pos: DVec3::ZERO,
            velocity: DVec3::new(0.1, 0.2, 0.3),
            orientation: DQuat::IDENTITY,
            omega: DVec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            mass: 1.0,
            charge: 0.0,
            color: Srgba::new(255, 0, 0, 255),
            ref_pos: DVec3::ZERO,
        });


    save_snapshot(dir_path, 1, &particles, 0.1)?;
    save_snapshot(dir_path, 10, &particles, 1.0)?; 

    let (_, latest_step, latest_time) = load_latest_snapshot(dir_path)?;

    //Check loads latest
    assert_eq!(latest_step, 10);
    assert_eq!(latest_time, 1.0);
    
    Ok(())
}
