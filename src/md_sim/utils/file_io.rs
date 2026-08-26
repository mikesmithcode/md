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
use std::{fs, path::Path, path::PathBuf, io};
use std::io::{Error, BufReader};
use polars::prelude::*;
use glam::{DVec3, DQuat};
use three_d::core::Srgba;
use itertools::izip;

use crate::md_sim::{Particle, ParticleVec, SimulationSettings, ObjectSpec, RectSpec, TriSpec};
use crate::md_viz::SceneSettings;


#[derive(Default)]
/// Encapsulates all major file paths required for running and saving a simulation.
pub struct SimulationPaths {
    pub output: PathBuf,
    pub sim_config: PathBuf,
    pub scene_config: PathBuf,
    pub object: PathBuf,
    pub particle: PathBuf,
    pub video: PathBuf,
}


/// Validates that a target directory exists and contains a specified list of required files.
///
/// # Arguments
/// * `dir_path` - A reference to the `Path` representing the directory to check.
/// * `required_files` - A slice of file name strings expected to be inside the directory.
///
/// # Errors
/// Returns an `io::Error` with `ErrorKind::NotFound` if the directory itself does not exist 
/// or if any of the required files are missing.
fn validate_simulation_inputs(dir_path: &Path, required_files: &[&str]) -> io::Result<()> {
    // 1. Check if the directory exists
    if !dir_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Required directory does not exist: {}", dir_path.display()),
        ));
    }

    // 2. Check each required file inside the folder
    for file_name in required_files {
        let file_path = dir_path.join(file_name);
        if !file_path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Required file missing: {}", file_path.display()),
            ));
        }
    }

    Ok(())
}

/// Parses command-line arguments, constructs input/output simulation filepaths, 
/// and validates that necessary configuration and initial snapshot files exist on disk.
///
/// Expects two command-line arguments:
/// 1. The simulation target name (e.g., `silo`)
/// 2. The specific run argument/identifier (e.g., `silo_123`)
///
/// Returns a `SimulationPaths` struct containing the organized `PathBuf` entries.
pub fn filepaths() -> SimulationPaths {
    // Grab command line arguments directly at the top of the function
    let args: Vec<String> = std::env::args().collect();
    
    // Expect the target name and simulation argument to be present
    let target_name = args.get(1).expect(
        "Error: No target name provided. \n\
         Usage: Please run via your shell script (e.g., `./run silo_123`) \n\
         or pass the target name and sim argument explicitly."
    );
    let sim_arg = args.get(2).expect(
        "Error: No simulation argument provided."
    );

    const INPUT_PATH: &'static str = "input";
    
    // Construct paths cleanly from the explicit components
    let config_path = Path::new(INPUT_PATH).join(target_name);
    let output_path = Path::new("output").join(target_name).join(sim_arg);

    // Run your validation checks
    let required_files = vec!["sim_settings.json", "scene_settings.json"];
    let _ = validate_simulation_inputs(&config_path, &required_files);
    let sim_config = config_path.join("sim_settings.json"); 
    let scene_config = config_path.join("scene_settings.json");

    let particle = output_path.join("particles");
    let _ = validate_simulation_inputs(&particle, &["particles_0000000000.parquet"]);

    let object = output_path.join("objects");
    if let Err(_e) = fs::create_dir_all(&object) {
        eprintln!("Error creating directory");
    };

    let video_dir = output_path.join("video");
    if let Err(_e) = fs::create_dir_all(&video_dir) {
        eprintln!("Error creating directory");
    };
    let video = video_dir.join(format!("{}.mp4", sim_arg));

    SimulationPaths {
        output: output_path,
        sim_config,
        scene_config,
        object,
        particle,
        video,
    }
}

//-------------------------------------------------------------
// Config of simulation
//-------------------------------------------------------------

/// Loads a JSON config file into a [`SimulationSettings`] struct and updates its start index.
/// 
/// # Arguments
/// * `sim_paths` - [`SimulationPaths`] struct containing all key filepaths.
/// * `index` - The simulation step index to update within the loaded settings. This is used so that if simulation is re-run the start index is indicated in the filename and SimulationSettings automatically
pub fn load_sim_settings(sim_paths: &SimulationPaths, index: usize) -> Result<SimulationSettings, Box<dyn std::error::Error>>
{   
    let file = fs::File::open(&sim_paths.sim_config)?;
    let reader = BufReader::new(file);
    
    let mut sim_settings: SimulationSettings = serde_json::from_reader(reader).expect("Does your config file match an enum variant in simulation.rs?");
    sim_settings.start = index;

    // Save a copy of config to output with simulation index as suffix.
    save_sim_settings(&sim_settings, &sim_paths.output)?;
    
    Ok(sim_settings)
}

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
pub fn save_sim_settings(sim_settings: &SimulationSettings, snapshot_path: &Path) -> Result<(), Error> 
{
    let sim_filename = format!("sim_config_{:010}.json", sim_settings.start);
    let full_filename = Path::new(&snapshot_path).join(sim_filename);
    let json = serde_json::to_string_pretty(sim_settings)
        .expect("Error serializing metadata");
    fs::write(full_filename, json)?;
    Ok(())
}

//--------------------------------------------------------
// Graphics config
//--------------------------------------------------------



/// Loads a JSON config file into a [`SceneSettings`] struct and updates its start index.
/// 
/// # Arguments
/// * `sim_paths` - [`SimulationPaths`] struct containing all key filepaths.
pub fn load_scene_settings(sim_paths: &SimulationPaths) -> Result<SceneSettings, Box<dyn std::error::Error>> {
        let file = fs::File::open(&sim_paths.scene_config)?;
        let reader = BufReader::new(file);

        let scene_settings: SceneSettings = serde_json::from_reader(reader).expect("Does your config file match an enum variant in simulation.rs?");
        
        let _ = save_scene_settings(&scene_settings, &sim_paths.output);
        Ok(scene_settings)
}

pub fn save_scene_settings(scene_settings: &SceneSettings, snapshot_path: &Path) -> Result<(), Error> 
{
    let output_filename = Path::new(&snapshot_path).join("scene_config.json");
    let json = serde_json::to_string_pretty(scene_settings)
        .expect("Error serializing metadata");
    fs::write(output_filename, json)?;
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
    dir_path: &SimulationPaths,
) -> Result<(ParticleVec, usize, f64), Box<dyn std::error::Error>> {
    if !dir_path.particle.exists() {
        return Err(format!("Particle directory does not exist: {}", dir_path.particle.display()).into());
    }

    let mut entries: Vec<(std::path::PathBuf, usize)> = fs::read_dir(&dir_path.particle)?
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
        if let Ok((particles, time)) = load_particles(&path) {
            return Ok((particles, step, time));
        } else {
            // If it failed, inspect the error to see if it's corruption
            match load_particles(&path) {
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
                _ => unreachable!(),
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
    sim_paths: &SimulationPaths,
    step: usize,
    particles: &ParticleVec,
    time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&sim_paths.particle)?;

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
    let final_path = &sim_paths.particle.join(&filename);
    
    let file = std::fs::File::create(&final_path)?;
    ParquetWriter::new(file).finish(&mut df)?;

    Ok(())
}

//---------------------------------------------------------
// Load and save objects - eg simbox
//---------------------------------------------------------

pub fn load_objects(
    sim_paths: &SimulationPaths,
    step: usize,
) -> Result<Vec<ObjectSpec>, Box<dyn std::error::Error>> {
    let filename = format!("objects_{:010}.parquet", step);
    let final_path = sim_paths.object.join(&filename);

    // Ensure the file exists before trying to read
    if !final_path.exists() {
        return Err(format!("Object snapshot file not found: {}", final_path.display()).into());
    }

    let file = std::fs::File::open(&final_path)?;
    let df = ParquetReader::new(file).finish()?;

    let mut objects = Vec::new();

    // Extract series for iteration
    let x1 = df.column("x1")?.f64()?;
    let y1 = df.column("y1")?.f64()?;
    let z1 = df.column("z1")?.f64()?;
    let x2 = df.column("x2")?.f64()?;
    let y2 = df.column("y2")?.f64()?;
    let z2 = df.column("z2")?.f64()?;
    let x3 = df.column("x3")?.f64()?;
    let y3 = df.column("y3")?.f64()?;
    let z3 = df.column("z3")?.f64()?;
    let x4 = df.column("x4")?.f64()?;
    let y4 = df.column("y4")?.f64()?;
    let z4 = df.column("z4")?.f64()?;

    let vx = df.column("vx")?.f64()?;
    let vy = df.column("vy")?.f64()?;
    let vz = df.column("vz")?.f64()?;
    let wx = df.column("wx")?.f64()?;
    let wy = df.column("wy")?.f64()?;
    let wz = df.column("wz")?.f64()?;

    let r = df.column("r")?.f64()?;
    let g = df.column("g")?.f64()?;
    let b = df.column("b")?.f64()?;
    let a = df.column("a")?.f64()?;

    let vis = df.column("visible")?.bool()?;

    let height = df.height();

    for i in 0..height {        
        let velocity = DVec3::new(
            vx.get(i).unwrap_or(0.0),
            vy.get(i).unwrap_or(0.0),
            vz.get(i).unwrap_or(0.0),
        );
        
        let omega = DVec3::new(
            wx.get(i).unwrap_or(0.0),
            wy.get(i).unwrap_or(0.0),
            wz.get(i).unwrap_or(0.0),
        );

        // Reconstruct Srgba color (assuming u8 values mapped from f64)
        let colour = Srgba {
            r: r.get(i).unwrap_or(0.0) as u8,
            g: g.get(i).unwrap_or(0.0) as u8,
            b: b.get(i).unwrap_or(0.0) as u8,
            a: a.get(i).unwrap_or(255.0) as u8,
        };

        let v1 = DVec3::new(x1.get(i).unwrap_or(0.0), y1.get(i).unwrap_or(0.0), z1.get(i).unwrap_or(0.0));
        let v2 = DVec3::new(x2.get(i).unwrap_or(0.0), y2.get(i).unwrap_or(0.0), z2.get(i).unwrap_or(0.0));
        let v3 = DVec3::new(x3.get(i).unwrap_or(0.0), y3.get(i).unwrap_or(0.0), z3.get(i).unwrap_or(0.0));

        let visible = vis.get(i).unwrap_or(true);
        
        let current_x4 = x4.get(i).unwrap_or(f64::NAN);

        if current_x4.is_nan() {
            // It's a triangle (3 vertices)
            // Note: You will need a TriSpec constructor matching your struct definition
            let mut tri_spec = TriSpec::new([v1, v2, v3], colour, visible);
            tri_spec.velocity = velocity;
            tri_spec.omega = omega;
             
            objects.push(ObjectSpec::Triangle(tri_spec));
        } else {
            // It's a rectangle (4 vertices)
            let v4 = DVec3::new(
                current_x4,
                y4.get(i).unwrap_or(0.0),
                z4.get(i).unwrap_or(0.0),
            );
            
            let mut rect_spec = RectSpec::new([v1, v2, v3, v4],colour,visible);
            rect_spec.velocity = velocity;
            rect_spec.omega = omega;

            objects.push(ObjectSpec::Rectangle(rect_spec));
        }
    }

    Ok(objects)
}

/// Finds and loads the latest valid object snapshot file from the specified simulation path directory.
/// 
/// Scans the directory for files matching `objects_*.parquet`, sorts them descending 
/// by step index, and loads the newest uncorrupted snapshot.
/// 
/// # Arguments
/// * `sim_paths` - Reference to the `SimulationPaths` struct containing directories.
/// 
/// # Returns
/// * `(Vec<ObjectSpec>, usize, f64)` - Vector of objects, step number, and simulation timestamp.
pub fn load_latest_objects(
    sim_paths: &SimulationPaths,
) -> Result<Option<Vec<ObjectSpec>>, Box<dyn std::error::Error>> {
    // If the object directory doesn't exist yet, it's safe to return None
    if !sim_paths.object.exists() {
        return Ok(None);
    }

    let mut entries: Vec<(std::path::PathBuf, usize)> = fs::read_dir(&sim_paths.object)?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let step = name.strip_prefix("objects_")?
                           .strip_suffix(".parquet")?
                           .parse::<usize>().ok()?;
            Some((entry.path(), step))
        })
        .collect();

    if entries.is_empty() {
        return Ok(None);
    }

    // Sort descending so the highest step index comes first
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    for (_path, step) in entries {
        if let Ok(objects) = load_objects(sim_paths, step) {
            return Ok(Some(objects));
        }
    }

    // If all files were corrupted or failed to load
    Ok(None)
}
    


pub fn save_objects(
    sim_paths: &SimulationPaths,
    step: usize,
    objects: Option<&[ObjectSpec]>,
    time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(all_objects) = objects{ 
    fs::create_dir_all(&sim_paths.object)?;

    let count = all_objects.len();
    let t: Vec<f64> = vec![time; count];
    let sentinel = f64::NAN;

    // Temporary vectors to accumulate properties across different object types
    let mut ids = Vec::with_capacity(count);
    let mut velocities = Vec::with_capacity(count);
    let mut omegas = Vec::with_capacity(count);
    let mut colours = Vec::with_capacity(count);
    
    // Vertex coordinate vectors (x1..x4, y1..y4, z1..z4)
    let mut x1 = Vec::with_capacity(count); let mut y1 = Vec::with_capacity(count); let mut z1 = Vec::with_capacity(count);
    let mut x2 = Vec::with_capacity(count); let mut y2 = Vec::with_capacity(count); let mut z2 = Vec::with_capacity(count);
    let mut x3 = Vec::with_capacity(count); let mut y3 = Vec::with_capacity(count); let mut z3 = Vec::with_capacity(count);
    let mut x4 = Vec::with_capacity(count); let mut y4 = Vec::with_capacity(count); let mut z4 = Vec::with_capacity(count);

    for obj in all_objects {
        match obj {
            ObjectSpec::Rectangle(rect) => {
                ids.push(rect.id as u64);
                velocities.push(rect.velocity);
                omegas.push(rect.omega);
                colours.push(rect.colour);

                // Rectangles have 4 vertices
                x1.push(rect.vertices[0].x); y1.push(rect.vertices[0].y); z1.push(rect.vertices[0].z);
                x2.push(rect.vertices[1].x); y2.push(rect.vertices[1].y); z2.push(rect.vertices[1].z);
                x3.push(rect.vertices[2].x); y3.push(rect.vertices[2].y); z3.push(rect.vertices[2].z);
                x4.push(rect.vertices[3].x); y4.push(rect.vertices[3].y); z4.push(rect.vertices[3].z);
            }
            ObjectSpec::Triangle(tri) => {
                ids.push(tri.id as u64);
                velocities.push(tri.velocity);
                omegas.push(tri.omega);
                colours.push(tri.colour);

                // Triangles have 3 vertices; pad the 4th vertex with NaN sentinel
                x1.push(tri.vertices[0].x); y1.push(tri.vertices[0].y); z1.push(tri.vertices[0].z);
                x2.push(tri.vertices[1].x); y2.push(tri.vertices[1].y); z2.push(tri.vertices[1].z);
                x3.push(tri.vertices[2].x); y3.push(tri.vertices[2].y); z3.push(tri.vertices[2].z);
                x4.push(sentinel);         y4.push(sentinel);         z4.push(sentinel);
            }
            ObjectSpec::WireBox(boxspec) => {
                // If wire boxes are handled as objects, you can unpack their vertices similarly, 
                // or handle them via an alternative representation if they use bounding boxes instead.
                // For now, we can log or handle them if needed.
                let _ = boxspec; 
            }
        }
    }

    let mut df = df!(
        "t" => &t,
        "id" => &ids,
        "x1" => &x1, "y1" => &y1, "z1" => &z1,
        "x2" => &x2, "y2" => &y2, "z2" => &z2,
        "x3" => &x3, "y3" => &y3, "z3" => &z3,
        "x4" => &x4, "y4" => &y4, "z4" => &z4,
        "vx" => &velocities.iter().map(|v| v.x).collect::<Vec<_>>(),
        "vy" => &velocities.iter().map(|v| v.y).collect::<Vec<_>>(),
        "vz" => &velocities.iter().map(|v| v.z).collect::<Vec<_>>(),
        "wx" => &omegas.iter().map(|w| w.x).collect::<Vec<_>>(),
        "wy" => &omegas.iter().map(|w| w.y).collect::<Vec<_>>(),
        "wz" => &omegas.iter().map(|w| w.z).collect::<Vec<_>>(),
        "r"  => &colours.iter().map(|c| c.r as f64).collect::<Vec<_>>(),
        "g"  => &colours.iter().map(|c| c.g as f64).collect::<Vec<_>>(),
        "b"  => &colours.iter().map(|c| c.b as f64).collect::<Vec<_>>(),
        "a"  => &colours.iter().map(|c| c.a as f64).collect::<Vec<_>>(),
    )?;

    let filename = format!("objects_{:010}.parquet", step);
    let final_path = sim_paths.object.join(&filename);
    
    let file = std::fs::File::create(&final_path)?;
    ParquetWriter::new(file).finish(&mut df)?;
    }else{
        println!("Skipping saving objects as Option(objects) = None");
    }
    Ok(())
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
