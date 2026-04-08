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


use crate::{Particle, ParticleVec};
use crate::md_sim::simulation::SimulationSettings;
use serde_json;
use std::{fs, io::Error, path::Path, path::PathBuf};

use std::io::BufReader;
use polars::prelude::*;
use glam::DVec3;
use three_d::core::Srgba;
use itertools::izip;

use crate::md_viz::scene::SceneSetup;

/// Generate all the filepaths
/// 
/// use the file!() macro as input. Do this [sim_config,scene_config, snapshot, video]=filepaths(file!());
/// This returns three filepaths of type Path
pub fn filepaths(script_name: &str)-> [PathBuf;4]{
    //Specify the folder in which all the output will be stored. Assumes in root of workspace.
    const OUTPUT_PATH: &'static str = "output";
    const INPUT_PATH: &'static str = "input";

    //----------------------------------------------------------------
    // Define simulation
    //---------------------------------------------------------------
    let simulation_name = Path::new(script_name)
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap();

    let sim_config_path = Path::new(INPUT_PATH).join(format!("{}.json", simulation_name));
    let scene_config_path = Path::new(INPUT_PATH).join("scene.json");

    let snapshot_path = Path::new(OUTPUT_PATH).join(simulation_name).join("snapshots");
    if let Err(_e) = fs::create_dir_all(&snapshot_path){
        eprintln!("Error creating directory");
    };
    let _ = snapshot_path.join("snapshots");

    let video_path = Path::new(OUTPUT_PATH).join(simulation_name).join("video");
    if let Err(_e) = fs::create_dir_all(&video_path){
        eprintln!("Error creating directory");
    };
    let video_path = video_path.join(simulation_name).with_extension("mp4");

    [sim_config_path, scene_config_path, snapshot_path, video_path]
}


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
pub fn save_simsettings(sim_settings: &SimulationSettings, snapshot_path: &Path) -> Result<(), Error> 
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
/// 
/// ```rust, ignore
/// pub struct SimulationSettings<T>{
///    pub dt: f64,
///    pub sim_box_size: [f64; 3], 
///    pub start: usize,
///    pub num_steps: usize,
///    pub sim_path: String,
///    pub dump: usize,
/// }
/// ```
/// If there are no extra parameters in the file
/// 
/// We update the start field to match index the initial value of the loop. Thus if you restart
/// simulation start will be at the correct value.
pub fn load_simsettings(input_filepath: &Path, output_path: &Path, index: usize) -> Result<SimulationSettings, Box<dyn std::error::Error>>
{   
    let file = fs::File::open(input_filepath)?;
    let reader = BufReader::new(file);
    
    let mut sim_settings: SimulationSettings = serde_json::from_reader(reader).expect("Does your config file match an enum variant in simulation.rs?");
    sim_settings.start = index;

    //Save a copy of config to output with simulation index as suffix.
    save_simsettings(&sim_settings, output_path)?;
    
    Ok(sim_settings)
}

/// saves particle snapshot to Parquet file
/// 
/// Its taking a `Vec<Particle>` and storing each field as an individual
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
    particles: &ParticleVec,
    time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create directory if it doesn't exist
    fs::create_dir_all(dir_path)?;


    let t: Vec<f64> = vec![time;particles.len()];
    let ids: Vec<u64> = particles.id.iter().map(|&id| id as u64).collect();
    let ptype: Vec<u64> = particles.ptype.iter().map(|&ptype| ptype as u64).collect();

    let mut df = df!(
        "t" => &t,
        "id" => &ids,
        "ptype" => &ptype,
        "x" => &particles.position.iter().map(|p| p.x).collect::<Vec<_>>(),
        "y" => &particles.position.iter().map(|p| p.y).collect::<Vec<_>>(),
        "z" => &particles.position.iter().map(|p| p.z).collect::<Vec<_>>(),
        "vx" => &particles.velocity.iter().map(|v| v.x).collect::<Vec<_>>(),
        "vy" => &particles.velocity.iter().map(|v| v.y).collect::<Vec<_>>(),
        "vz" => &particles.velocity.iter().map(|v| v.z).collect::<Vec<_>>(),
        "radius" => &particles.radius,
        "mass" => &particles.mass,
        "r" => &particles.color.iter().map(|c| c.r as f64).collect::<Vec<_>>(),
        "g" => &particles.color.iter().map(|c| c.g as f64).collect::<Vec<_>>(),
        "b" => &particles.color.iter().map(|c| c.b as f64).collect::<Vec<_>>(),
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
/// Load particle snapshot from Parquet file into a ParticleVec
pub fn load_snapshot(
    file_path: &Path,
) -> Result<(ParticleVec, f64), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let df = ParquetReader::new(file).finish()?;

    let count = df.height();
    let mut particles = ParticleVec::with_capacity(count);

    let t_col = df.column("t")?.f64()?;
    let id_col = df.column("id")?.u64()?;
    let ptype_col = df.column("ptype")?.u64()?;
    let x_col = df.column("x")?.f64()?;
    let y_col = df.column("y")?.f64()?;
    let z_col = df.column("z")?.f64()?;
    let vx_col = df.column("vx")?.f64()?;
    let vy_col = df.column("vy")?.f64()?;
    let vz_col = df.column("vz")?.f64()?;
    let r_col = df.column("radius")?.f64()?;
    let m_col = df.column("mass")?.f64()?;
    let col_r = df.column("r")?.f64()?;
    let col_g = df.column("g")?.f64()?;
    let col_b = df.column("b")?.f64()?;

    
    let t = t_col.get(0).unwrap_or(0.0);

    // Efficiently populate the ParticleVec
    // We use izip! to iterate through all columns simultaneously
    for (id, ptype, x, y, z, vx, vy, vz, rad, mass, r, g, b) in izip!(
        id_col.into_iter(),
        ptype_col.into_iter(),
        x_col.into_iter(),
        y_col.into_iter(),
        z_col.into_iter(),
        vx_col.into_iter(),
        vy_col.into_iter(),
        vz_col.into_iter(),
        r_col.into_iter(),
        m_col.into_iter(),
        col_r.into_iter(),
        col_g.into_iter(),
        col_b.into_iter()
    ) {
        // We use .unwrap_or because Polars columns are technically nullable
        particles.push(Particle {
            id: id.unwrap_or(0) as usize,
            ptype: ptype.unwrap_or(0) as usize,
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
            radius: rad.unwrap_or(0.0),
            mass: mass.unwrap_or(0.0),
            color: Srgba::new(
                r.unwrap_or(0.0) as u8,
                g.unwrap_or(0.0) as u8,
                b.unwrap_or(0.0) as u8,
                255, // Full opacity
            ),
            ref_pos: DVec3::ZERO,
        });
    }

    Ok((particles, t))
}

/// Load the latest snapshot from a directory
/// 
/// Searches files in output/snapshots for the latest
/// set of particle positions and then uses load_snapshot to
/// generate `Vec<Particle>`, simulation index and simulation time.
/// 
/// # Arguments
/// * `dir_path` - Directory containing snapshot files
/// 
/// # Returns
/// * `(particles, step, time)` - Vector of particles, step number, and simulation time
pub fn load_latest_snapshot(
    dir_path: &Path,
) -> Result<(ParticleVec, usize, f64), Box<dyn std::error::Error>> {
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

pub fn load_scene_settings<P: AsRef<Path>>(path: P) -> Result<SceneSetup, Box<dyn std::error::Error>> {
    // Open the file in read-only mode
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    // Deserialise the JSON into the struct
    let settings = serde_json::from_reader(reader)?;

    Ok(settings)
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
        let mut particles = ParticleVec::new();
        particles.push(
            Particle {
                id: 1,
                ptype: 0,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                radius: 0.5,
                mass: 1.0,
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
                ptype: 0,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                radius: 0.5,
                mass: 1.0,
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
}
