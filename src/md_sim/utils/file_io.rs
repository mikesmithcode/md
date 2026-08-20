//! Input and output of simulation states and config.
//!
//! This module handles two primary formats:
//!
//! 1. **Metadata (JSON)**: Handled by [`save_simsettings`] and [`load_simsettings`], 
//!    this stores and loads the parameters of the experiment (e.g., time step, box size, 
//!    simulation models).
//! 2. **State Snapshots (Parquet)**: Handled by [`save_particles`] and [`load_particles`], 
//!    this uses the **Polars** library to efficiently store particle positions, velocities, 
//!    orientations, angular velocities, and optical properties.
//!
//! ### Data Workflow
//! The simulation periodically saves snapshots. These files can be reloaded 
//! using [`load_latest_particles`] to resume a previously stopped experiment.

use serde_json;
use std::{fs, path::Path, path::PathBuf};

use std::io::{Error, BufReader};
use polars::prelude::*;
use glam::{DVec3, DQuat};
use three_d::core::Srgba;
use itertools::izip;

use crate::md_sim::{Particle, ParticleVec, SimulationSettings, ObjectSpec};
use crate::md_viz::SceneSettings;


/// Generate all the filepaths for simulation inputs, snapshots, and video outputs.
/// 
/// Returns an array of five `PathBuf` elements: `[sim_config_path, scene_config_path, 
/// object_snapshot_path, particle_snapshot_path, video_path]`.
pub fn filepaths() -> [PathBuf; 5] {
    // Grab command line arguments directly at the top of the function
    let args: Vec<String> = std::env::args().collect();
    
    // Expect the argument to be present; panic with an informative message if missing
    let run_dir = args.get(1).expect(
        "Error: No run directory provided. \n\
         Usage: Please run via your shell script (e.g., `./run silo_123`) \n\
         or pass the output directory argument explicitly."
    );

    const INPUT_PATH: &'static str = "input";
    let run_path = Path::new(run_dir);

    // Extract the simulation target name (e.g., "silo") from the parent directory
    let simulation_name = run_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .expect("Error: Invalid run directory path structure. Expected something like 'output/target/target_id'.");

    // Extract the specific simulation argument (e.g., "silo_123") from the final component
    let sim_arg = run_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("Error: Could not extract simulation run name from path.");

    let sim_config_path = Path::new(INPUT_PATH).join(format!("{}.json", simulation_name));
    let scene_config_path = Path::new(INPUT_PATH).join("scene_settings.json");

    let particle_snapshot_path = run_path.join("particles");
    if let Err(_e) = fs::create_dir_all(&particle_snapshot_path) {
        eprintln!("Error creating directory");
    };

    let object_snapshot_path = run_path.join("objects");
    if let Err(_e) = fs::create_dir_all(&object_snapshot_path) {
        eprintln!("Error creating directory");
    };

    let video_dir = run_path.join("video");
    if let Err(_e) = fs::create_dir_all(&video_dir) {
        eprintln!("Error creating directory");
    };
    let video_path = video_dir.join(format!("{}.mp4", sim_arg));

    [
        sim_config_path,
        scene_config_path,
        object_snapshot_path,
        particle_snapshot_path,
        video_path,
    ]
}


//-------------------------------------------------------------
// Config of simulation
//-------------------------------------------------------------

/// Saves a JSON representation of the current [`SimulationSettings`]. 
/// 
/// The configuration is serialized and saved into the provided path directory.
///
/// # File Naming
/// The filename is automatically generated using the `start` step counter to ensure 
/// uniqueness (e.g., `sim_config_0000000001.json`).
///
/// # Errors
/// This function will return an [`Error`] if the directory is not writable 
/// or if an I/O issue occurs during writing.
pub fn save_simsettings(sim_settings: &SimulationSettings, snapshot_path: &Path) -> Result<(), Error> 
{
    let filename = format!("sim_config_{:010}.json", sim_settings.start);
    let full_filename = Path::new(&snapshot_path).join(filename);
    let json = serde_json::to_string_pretty(sim_settings)
        .expect("Error serializing metadata");
    fs::write(full_filename, json)?;
    Ok(())
}

/// Loads a JSON config file into a [`SimulationSettings`] struct and updates its start index.
/// 
/// # Arguments
/// * `input_filepath` - Path to the source JSON configuration file.
/// * `output_path` - Directory where a copy of the loaded config should be archived.
/// * `index` - The simulation step index to update within the loaded settings.
pub fn load_simsettings(input_filepath: &Path, output_path: &Path, index: usize) -> Result<SimulationSettings, Box<dyn std::error::Error>>
{   
    let file = fs::File::open(input_filepath)?;
    let reader = BufReader::new(file);
    
    let mut sim_settings: SimulationSettings = serde_json::from_reader(reader).expect("Does your config file match an enum variant in simulation.rs?");
    sim_settings.start = index;

    // Save a copy of config to output with simulation index as suffix.
    save_simsettings(&sim_settings, output_path)?;
    
    Ok(sim_settings)
}

//---------------------------------------------------------
// Load and save objects - eg simbox
//---------------------------------------------------------
pub fn load_objects(file_path: &Path)-> Result<(), Box<dyn std::error::Error>>{
    let _file = std::fs::File::open(file_path)?;
    Ok(()) 
}

pub fn save_objects(_dir_path: &Path,
    _step: usize,
    _objects: &[ObjectSpec],
    _time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())   
}


//--------------------------------------------------------
// Load and save particles
//--------------------------------------------------------

/// Loads particle snapshot data from a Parquet file into a [`ParticleVec`] and returns simulation time.
/// 
/// Each row in the file represents a particle, mapped to fields within the [`Particle`] struct.
/// Missing optional columns are populated with standard default values.
/// 
/// # Arguments
/// * `file_path` - Path to the particle Parquet snapshot file.
/// 
/// # Returns
/// * `(ParticleVec, f64)` - The vector containing populated particles and the simulation timestamp.
pub fn load_particles(file_path: &Path) -> Result<(ParticleVec, f64), Box<dyn std::error::Error>> {
    println!("load_particles {:?}", file_path );
    let file = std::fs::File::open(file_path)?;
    let df = ParquetReader::new(file).finish()?;

    let count = df.height();
    let mut particles = ParticleVec::with_capacity(count);

    let t_col = df.column("t")?.f64()?;
    let id_col = df.column("id")?.u64()?;
    let molecule_id_col = get_u64_col_or_id(&df)?.u64()?.clone();
    let ptype_series = get_u64_col_or_filler(&df, "ptype", 0 as u64);
    let ptype_col = ptype_series.u64()?;
    let x_col = df.column("x")?.f64()?;
    let y_col = df.column("y")?.f64()?;
    let z_col = df.column("z")?.f64()?;
    let rel_x_series = get_f64_col(&df, "rel_x", 0.0);
    let rel_x_col = rel_x_series.f64()?;
    let rel_y_series = get_f64_col(&df, "rel_y", 0.0);
    let rel_y_col = rel_y_series.f64()?;
    let rel_z_series = get_f64_col(&df, "rel_z", 0.0);
    let rel_z_col = rel_z_series.f64()?;
    let vx_series = get_f64_col(&df, "vx", 0.0);
    let vx_col = vx_series.f64()?;
    let vy_series = get_f64_col(&df, "vy", 0.0);
    let vy_col = vy_series.f64()?;
    let vz_series = get_f64_col(&df, "vz", 0.0);
    let vz_col = vz_series.f64()?;
    let qx_series = get_f64_col(&df, "qx", 0.0);
    let qx_col = qx_series.f64()?;
    let qy_series = get_f64_col(&df, "qy", 0.0);
    let qy_col = qy_series.f64()?;
    let qz_series = get_f64_col(&df, "qz", 0.0);
    let qz_col = qz_series.f64()?;
    let qw_series = get_f64_col(&df, "qw", 1.0);
    let qw_col = qw_series.f64()?;
    let wx_series = get_f64_col(&df, "wx", 0.0);
    let wx_col = wx_series.f64()?;
    let wy_series = get_f64_col(&df, "wy", 0.0);
    let wy_col = wy_series.f64()?;
    let wz_series = get_f64_col(&df, "wz", 0.0);
    let wz_col = wz_series.f64()?;
    let r_col = df.column("radius")?.f64()?;
    let m_col = df.column("mass")?.f64()?;
    let q_series = get_f64_col(&df, "charge", 0.0);
    let q_col = q_series.f64()?;
    let visible_series = get_bool_col(&df, "visible", true);
    let visible_col = visible_series.bool()?;
    let r_series = get_f64_col(&df, "r", 255.0);
    let col_r = r_series.f64()?;
    let g_series = get_f64_col(&df, "g", 0.0);
    let col_g = g_series.f64()?;
    let b_series = get_f64_col(&df, "b", 0.0);
    let col_b = b_series.f64()?;
    let a_series = get_f64_col(&df, "a", 255.0);
    let col_a = a_series.f64()?;

    let t = t_col.get(0).unwrap_or(0.0);

    for (id, molecule_id, ptype, x, y, z, rel_x, rel_y, rel_z, vx, vy, vz, qx, qy, qz, qw, wx, wy, wz, rad, mass, charge, visible, r, g, b, a) in izip!(
        id_col.into_iter(),
        molecule_id_col.into_iter(),
        ptype_col.into_iter(),
        x_col.into_iter(),
        y_col.into_iter(),
        z_col.into_iter(),
        rel_x_col.into_iter(),
        rel_y_col.into_iter(),
        rel_z_col.into_iter(),
        vx_col.into_iter(),
        vy_col.into_iter(),
        vz_col.into_iter(),
        qx_col.into_iter(),
        qy_col.into_iter(),
        qz_col.into_iter(),
        qw_col.into_iter(),
        wx_col.into_iter(),
        wy_col.into_iter(),
        wz_col.into_iter(),
        r_col.into_iter(),
        m_col.into_iter(),
        q_col.into_iter(),
        visible_col.into_iter(),
        col_r.into_iter(),
        col_g.into_iter(),
        col_b.into_iter(),
        col_a.into_iter()
    ) {
        particles.push(Particle {
            id: id.unwrap_or(0) as usize,
            molecule_id: molecule_id.unwrap_or(0) as usize,
            ptype: ptype.unwrap_or(0) as usize,
            position: DVec3::new(
                x.unwrap_or(0.0),
                y.unwrap_or(0.0),
                z.unwrap_or(0.0),
            ),
            rel_pos: DVec3::new(
                rel_x.unwrap_or(0.0),
                rel_y.unwrap_or(0.0),
                rel_z.unwrap_or(0.0),
            ),
            velocity: DVec3::new(
                vx.unwrap_or(0.0),
                vy.unwrap_or(0.0),
                vz.unwrap_or(0.0),
            ),
            orientation: DQuat::from_xyzw(
                qx.unwrap_or(0.0),
                qy.unwrap_or(0.0),
                qz.unwrap_or(0.0),
                qw.unwrap_or(1.0)
            ).normalize(),
            omega: DVec3::new(
                wx.unwrap_or(0.0),
                wy.unwrap_or(0.0),
                wz.unwrap_or(0.0),
            ),
            radius: rad.unwrap_or(0.0),
            mass: mass.unwrap_or(0.0),
            charge: charge.unwrap_or(0.0),
            visible: visible.unwrap_or(true),
            colour: Srgba::new(
                r.unwrap_or(0.0) as u8,
                g.unwrap_or(0.0) as u8,
                b.unwrap_or(0.0) as u8,
                a.unwrap_or(255.0) as u8,
            ),
            ref_pos: DVec3::ZERO,        
        });
    }
    
    Ok((particles, t))
}

/// Finds and loads the latest valid particle snapshot file from the specified directory.
/// 
/// Scans the directory for files matching `particles_*.parquet`, sorts them descending 
/// by step index, and loads the newest uncorrupted snapshot.
/// 
/// # Arguments
/// * `dir_path` - Directory path containing particle snapshot files.
/// 
/// # Returns
/// * `(ParticleVec, usize, f64)` - Vector of particles, step number, and simulation timestamp.
pub fn load_latest_particles(
    dir_path: &Path,
) -> Result<(ParticleVec, usize, f64), Box<dyn std::error::Error>> {
    let mut entries: Vec<(std::path::PathBuf, usize)> = fs::read_dir(dir_path)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let step = name.strip_prefix("particles_")?
                           .strip_suffix(".parquet")?
                           .parse::<usize>().ok()?;
            Some((entry.path(), step))
        })
        .collect();

    if entries.is_empty() {
        return Err("No snapshot files found".into());
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, step) in entries {
        match load_particles(&path) {
            Ok((particles, time)) => {
                return Ok((particles, step, time));
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("PAR1") || err_msg.contains("parquet") {
                    eprintln!("Warning: Corrupted snapshot found at {:?}. Removing and trying previous...", path);
                    if let Err(del_err) = fs::remove_file(&path) {
                        eprintln!("Failed to delete corrupted file: {}", del_err);
                    }
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err("All available snapshot files were corrupted or could not be read".into())
}

/// Saves a [`ParticleVec`] snapshot to a Parquet file.
/// 
/// Flattens the particle structure fields into individual columns and writes 
/// them to a Parquet file named `particles_{step:010}.parquet` within the target directory.
/// 
/// # Arguments
/// * `dir_path` - Target directory to save the snapshot.
/// * `step` - The current index of the simulation loop.
/// * `particles` - Reference to the [`ParticleVec`] collection.
/// * `time` - Current simulation time timestamp.
pub fn save_particles(
    dir_path: &Path,
    step: usize,
    particles: &ParticleVec,
    time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir_path)?;

    let t: Vec<f64> = vec![time; particles.len()];
    let id: Vec<u64> = particles.id.iter().map(|&id| id as u64).collect();
    let molecule_id: Vec<u64> = particles.molecule_id.iter().map(|&molecule_id| molecule_id as u64).collect();
    let ptype: Vec<u64> = particles.ptype.iter().map(|&ptype| ptype as u64).collect();

    let mut df = df!(
        "t" => &t,
        "id" => &id,
        "molecule_id" => &molecule_id,
        "ptype" => &ptype,
        "x" => &particles.position.iter().map(|p| p.x).collect::<Vec<_>>(),
        "y" => &particles.position.iter().map(|p| p.y).collect::<Vec<_>>(),
        "z" => &particles.position.iter().map(|p| p.z).collect::<Vec<_>>(),
        "rel_x" => &particles.rel_pos.iter().map(|p| p.x).collect::<Vec<_>>(),
        "rel_y" => &particles.rel_pos.iter().map(|p| p.y).collect::<Vec<_>>(),
        "rel_z" => &particles.rel_pos.iter().map(|p| p.z).collect::<Vec<_>>(),
        "vx" => &particles.velocity.iter().map(|v| v.x).collect::<Vec<_>>(),
        "vy" => &particles.velocity.iter().map(|v| v.y).collect::<Vec<_>>(),
        "vz" => &particles.velocity.iter().map(|v| v.z).collect::<Vec<_>>(),
        "qx" => &particles.orientation.iter().map(|q| q.x).collect::<Vec<_>>(),
        "qy" => &particles.orientation.iter().map(|q| q.y).collect::<Vec<_>>(),
        "qz" => &particles.orientation.iter().map(|q| q.z).collect::<Vec<_>>(),
        "qw" => &particles.orientation.iter().map(|q| q.w).collect::<Vec<_>>(),
        "wx" => &particles.omega.iter().map(|w| w.x).collect::<Vec<_>>(),
        "wy" => &particles.omega.iter().map(|w| w.y).collect::<Vec<_>>(),
        "wz" => &particles.omega.iter().map(|w| w.z).collect::<Vec<_>>(),
        "radius" => &particles.radius,
        "mass" => &particles.mass,
        "charge" => &particles.charge,
        "visible" => &particles.visible,
        "r" => &particles.colour.iter().map(|c| c.r as f64).collect::<Vec<_>>(),
        "g" => &particles.colour.iter().map(|c| c.g as f64).collect::<Vec<_>>(),
        "b" => &particles.colour.iter().map(|c| c.b as f64).collect::<Vec<_>>(),
        "a" => &particles.colour.iter().map(|c| c.a as f64).collect::<Vec<_>>(),
    )?;

    let filename = format!("particles_{:010}.parquet", step);
    let final_path = dir_path.join(&filename);
    
    let file = std::fs::File::create(&final_path)?;
    ParquetWriter::new(file).finish(&mut df)?;

    Ok(())
}

//--------------------------------------------------------
// Graphics config
//--------------------------------------------------------

/// Loads the JSON configuration file into a [`SceneSettings`] struct controlling visual properties.
pub fn load_scene_settings<P: AsRef<Path>>(path: P) -> Result<SceneSettings, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let settings = serde_json::from_reader(reader)?;
    Ok(settings)
}

//-----------------------------------------------------
// private helpers
//-----------------------------------------------------

fn get_u64_col_or_id(df: &DataFrame) -> PolarsResult<Series> {
    match df.column("molecule_id") {
        Ok(col) => Ok(col.clone()),
        Err(_) => {
            Ok(df.column("id")?.clone())
        }
    }
}

fn get_u64_col_or_filler(df: &DataFrame, name: &str, filler: u64) -> Series {
    df.column(name)
        .cloned()
        .unwrap_or_else(|_| {
            UInt64Chunked::full(name, filler, df.height()).into_series()
        })
}

fn get_f64_col(df: &DataFrame, name: &str, filler: f64) -> Series {
    df.column(name)
        .cloned()
        .unwrap_or_else(|_| {
            Float64Chunked::full(name, filler, df.height()).into_series()
        })
}

fn get_bool_col(df: &DataFrame, name: &str, filler: bool) -> Series {
    df.column(name)
        .cloned()
        .unwrap_or_else(|_| {
            BooleanChunked::full(name, filler, df.height()).into_series()
        })
}