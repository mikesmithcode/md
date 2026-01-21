use crate::{Particle, simulation::SimulationSettings};
use serde_json;
use std::{fs, io::Error, path::Path};
use polars::prelude::*;
use glam::DVec3;
use three_d::core::Srgba;

/// Save simulation metadata to JSON
pub fn save_simsettings(sim_settings: &SimulationSettings) -> Result<(), Error> {
    let filename = format!("sim_config_{:010}.json", sim_settings.start);
    let full_filename = Path::new(sim_settings.sim_path).join(filename);
    let json = serde_json::to_string_pretty(sim_settings)
        .expect("Error serializing metadata");
    fs::write(full_filename, json)?;
    Ok(())
}

//pub fn load_simsettings()

/// Save particle snapshot to Parquet file
/// 
/// # Arguments
/// * `dir_path` - Directory to save snapshots in
/// * `timestep` - Timestep number (for filename)
/// * `particles` - Vector of particles to save
/// * `time` - Simulation time (stored as metadata)
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
    let r = df.column("r")?.f64()?.into_iter().collect::<Vec<_>>();
    let g = df.column("g")?.f64()?.into_iter().collect::<Vec<_>>();
    let b = df.column("b")?.f64()?.into_iter().collect::<Vec<_>>();

    // Reconstruct particles
    let particles = ids
        .into_iter()
        .zip(x.into_iter().zip(y.into_iter().zip(z.into_iter())))
        .zip(vx.into_iter().zip(vy.into_iter().zip(vz.into_iter())))
        .zip(radius.into_iter().zip(r.into_iter().zip(g.into_iter().zip(b.into_iter()))))
        .map(|(((id, (x, (y, z))), (vx, (vy, vz))), (radius, (r, (g, b))))| {
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
