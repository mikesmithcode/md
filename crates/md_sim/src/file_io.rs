//! input and output of simulation states and config.
//!
//! This module handles two primary formats:
//!
//! 1. **Metadata (JSON)**: Handled by [`save_simsettings`], this stores / loads the 
//!    parameters of the experiment (e.g., simulation path, start time).
//! 2. **State Snapshots (Parquet)**: Handled by [`save_snapshot`] and [`load_snapshot`], 
//!    this uses the **Polars** library to efficiently store particle positions, 
//!    velocities, and properties.
//!
//! ### Data Workflow
//! The simulation periodically saves snapshots. These files can be reloaded 
//! using [`load_latest_snapshot`] to resume a previously stopped experiment.


use crate::{Particle, simulation::SimulationSettings};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_json;
use std::{fs, io::Error, path::Path};
use std::io::BufReader;
use polars::prelude::*;
use glam::DVec3;
use three_d::core::Srgba;
use itertools::izip;


/// saves a json representation of the current [`SimulationSettings`]. 
/// 
/// The info is serialised and saved as json.
///
/// # File Naming
/// The filename is automatically generated using the `start` timestamp to ensure 
/// uniqueness (e.g., `sim_config_0000000001.json`).
///
/// # Errors
/// This function will return an [`Error`] if:
/// * The `sim_path` directory does not exist or is not writable.
/// * There is an underlying I/O issue when writing to the disk.
///
/// # Panics
/// Panics if the `SimulationSettings` cannot be converted to JSON.
pub fn save_simsettings<T: Serialize>(sim_settings: &SimulationSettings<T>, snapshot_path: &Path) -> Result<(), Error> 
{
    let filename = format!("sim_config_{:010}.json", sim_settings.start);
    let full_filename = Path::new(&snapshot_path).join(filename);
    let json = serde_json::to_string_pretty(sim_settings)
        .expect("Error serializing metadata");
    fs::write(full_filename, json)?;
    Ok(())
}


/// loads a json config file into a SimulationSettings struct
/// 
/// SimulationSettings has standard fields and a catch all which would require 
/// writing a new struct type in you example and :
/// pub struct SimulationSettings<T>{
///    pub dt: f64,
///    pub sim_box_size: [f64; 3], 
///    pub start: usize,
///    pub num_steps: usize,
///    pub sim_path: String,
///    pub dump: usize,
///    // Special values
///    #[serde(flatten)]
///    pub extra: T,
///}
/// 
/// If there are no extra parameters in the file
/// 
/// We update the start field to match index the initial value of the loop. Thus if you restart
/// simulation start will be at the correct value.
pub fn load_simsettings<T>(input_filepath: &Path, output_path: &Path, index: usize) -> Result<SimulationSettings<T>, Box<dyn std::error::Error>>
where
    T: DeserializeOwned + Serialize,
{   

    let file = fs::File::open(input_filepath)?;
    let reader = BufReader::new(file);
    
    let mut sim_settings: SimulationSettings<T> = serde_json::from_reader(reader)?;
    sim_settings.start = index;

    //Save a copy of config to output with simulation index as suffix.
    save_simsettings::<T>(&sim_settings, output_path)?;
    
    Ok(sim_settings)
}

/// saves particle snapshot to Parquet file
/// 
/// Its taking a Vec<Particle> and storing each field as an individual
/// column in a Parquet file in output/snapshots.
/// 
/// # Arguments
/// * `dir_path` - Directory to save snapshots in
/// * `step` - the index of the simulation loop
/// * `particles` - Vector of particles to save
/// * `time` - Simulation time
pub fn save_snapshot(
    dir_path: &Path,
    step: usize,
    particles: &[Particle],
    time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create directory if it doesn't exist
    fs::create_dir_all(dir_path)?;

    // Extract data from particles
    let t: Vec<f64> = vec![time;particles.len()];
    let ids: Vec<u64> = particles.iter().map(|p| p.id as u64).collect();
    let x: Vec<f64> = particles.iter().map(|p| p.position.x).collect();
    let y: Vec<f64> = particles.iter().map(|p| p.position.y).collect();
    let z: Vec<f64> = particles.iter().map(|p| p.position.z).collect();
    let vx: Vec<f64> = particles.iter().map(|p| p.velocity.x).collect();
    let vy: Vec<f64> = particles.iter().map(|p| p.velocity.y).collect();
    let vz: Vec<f64> = particles.iter().map(|p| p.velocity.z).collect();
    let radius: Vec<f64> = particles.iter().map(|p| p.radius).collect();
    let inv_mass: Vec<f64> = particles.iter().map(|p| p.inv_mass).collect();
    let r: Vec<f64> = particles.iter().map(|p| p.color.r as f64).collect();
    let g: Vec<f64> = particles.iter().map(|p| p.color.g as f64).collect();
    let b: Vec<f64> = particles.iter().map(|p| p.color.b as f64).collect();

    // Create DataFrame
    let mut df = df!(
        "t" => &t,
        "id" => &ids,
        "x" => &x,
        "y" => &y,
        "z" => &z,
        "vx" => &vx,
        "vy" => &vy,
        "vz" => &vz,
        "radius" => &radius,
        "inv_mass" => &inv_mass,
        "r" => &r,
        "g" => &g,
        "b" => &b,
    )?;

    // Write to Parquet (with temp file for safety)
    let filename = format!("snapshot_{:010}.parquet", step);
    let temp_filename = format!("snapshot_{:010}.parquet.tmp", step);

    let temp_path = dir_path.join(&temp_filename);
    let final_path = dir_path.join(&filename);

    // Write to temporary file first with metadata
    {
        let file = std::fs::File::create(&temp_path)?;
        ParquetWriter::new(file).finish(&mut df)?;
    }

    // Atomic rename
    fs::rename(&temp_path, &final_path)?;

    Ok(())
}

/// Load particle snapshot from Parquet file
/// 
/// Each row in file represents a particle. Each column is a field
/// to be added the Particle struct. These are combined in a Vec.
/// 
/// # Arguments
/// * `file_path` - Path to the snapshot file
/// 
/// # Returns
/// * `(particles, time)` - Vector of particles and simulation time
pub fn load_snapshot(
    file_path: &Path,
) -> Result<(Vec<Particle>, f64), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let df = ParquetReader::new(file).finish()?;

    // Extract columns
    let time = df.column("t")?.f64()?.into_iter().collect::<Vec<_>>();
    let ids = df.column("id")?.u64()?.into_iter().collect::<Vec<_>>();
    let x = df.column("x")?.f64()?.into_iter().collect::<Vec<_>>();
    let y = df.column("y")?.f64()?.into_iter().collect::<Vec<_>>();
    let z = df.column("z")?.f64()?.into_iter().collect::<Vec<_>>();
    let vx = df.column("vx")?.f64()?.into_iter().collect::<Vec<_>>();
    let vy = df.column("vy")?.f64()?.into_iter().collect::<Vec<_>>();
    let vz = df.column("vz")?.f64()?.into_iter().collect::<Vec<_>>();
    let radius = df.column("radius")?.f64()?.into_iter().collect::<Vec<_>>();
    let inv_mass: Vec<Option<f64>> = df.column("inv_mass")?.f64()?.into_iter().collect::<Vec<_>>();
    let r = df.column("r")?.f64()?.into_iter().collect::<Vec<_>>();
    let g = df.column("g")?.f64()?.into_iter().collect::<Vec<_>>();
    let b = df.column("b")?.f64()?.into_iter().collect::<Vec<_>>();

    // Reconstruct particles
    let particles = izip!(ids,x,y,z,vx,vy,vz,radius,inv_mass,r,g,b)
        .map(|(id,x,y,z,vx,vy,vz,radius,inv_mass,r,g,b)| {
            Particle {
                id: id.unwrap_or(0) as usize,
                position: DVec3::new(
                    x.unwrap_or(0.0),
                    y.unwrap_or(0.0),
                    z.unwrap_or(0.0),
                ),
                velocity: DVec3::new(
                    vx.unwrap_or(0.0),
                    vy.unwrap_or(0.0),
                    vz.unwrap_or(0.0),
                ),
                radius: radius.unwrap_or(0.0),
                inv_mass: inv_mass.unwrap_or(0.0),
                color: Srgba::new(
                    r.unwrap_or(0.0) as u8,
                    g.unwrap_or(0.0) as u8,
                    b.unwrap_or(0.0) as u8,
                    255,
                ),
                
            }
        })
        .collect();

    // For now, return time as 0.0 (could add to DataFrame metadata if needed)
    Ok((particles, time[0].unwrap()))
}


/// Load the latest snapshot from a directory
/// 
/// Searches files in output/snapshots for the latest
/// set of particle positions and then uses load_snapshot to
/// generate Vec<Particle>, simulation index and simulation time.
/// 
/// # Arguments
/// * `dir_path` - Directory containing snapshot files
/// 
/// # Returns
/// * `(particles, step, time)` - Vector of particles, step number, and simulation time
pub fn load_latest_snapshot(
    dir_path: &Path,
) -> Result<(Vec<Particle>, usize, f64), Box<dyn std::error::Error>> {
    let (latest_path, latest_step) = fs::read_dir(dir_path)?
        .flatten() // Ignore entries we can't read
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let step = name.strip_prefix("snapshot_")?
                           .strip_suffix(".parquet")?
                           .parse::<usize>().ok()?;
            Some((entry.path(), step))
        })
        .max_by_key(|&(_, step)| step) // Find the entry with the highest step
        .ok_or("No snapshot files found")?;

    let (particles, time) = load_snapshot(&latest_path)?;
    Ok((particles, latest_step, time))
}




// Tests for file_io
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        // Setup temporary workspace
        let dir = tempdir()?;
        let dir_path = dir.path();
        
        // Create dummy particle data
        let particles = vec![
            Particle {
                id: 1,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                radius: 0.5,
                color: Srgba::new(255, 0, 0, 255),
            }
        ];
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
        assert_eq!(loaded_particles[0].id, 1);
        assert_eq!(loaded_time, 0.5);
        assert!((loaded_particles[0].position.x - 1.0).abs() < f64::EPSILON);
        
        Ok(())
    }

    #[test]
    fn test_load_latest_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let dir_path = dir.path();
        
        // Save two snapshots with different steps
        let particles = vec![]; 
        save_snapshot(dir_path, 1, &particles, 0.1)?;
        save_snapshot(dir_path, 10, &particles, 1.0)?; 

        let (_, latest_step, latest_time) = load_latest_snapshot(dir_path)?;

        //Check loads latest
        assert_eq!(latest_step, 10);
        assert_eq!(latest_time, 1.0);
        
        Ok(())
    }
}
