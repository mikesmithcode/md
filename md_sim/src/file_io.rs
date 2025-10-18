use crate::simulation::*;
use serde_json;
use std::{fs, io::Error};
use polars::prelude::*;

///Used to save the SimulationSettings struct to a metadata file.
pub fn save_metadata(filename: String, sim_settings: SimulationSettings)-> Result<(), Error>{
    let json = serde_json::to_string_pretty(&sim_settings).expect("Error serializing SimulationSettings");
    fs::write(filename, json);
    Ok(())
}

/*
pub fn save_particledata(filename: String, particles: Vec<Particle>){
    let mut df = df!(
    "foo" => &[1, 2, 3],
    "bar" => &[None, Some("bak"), Some("baz")],
    )
    .unwrap();

    let mut file = std::fs::File::create("docs/assets/data/path.parquet").expect();
    ParquetWriter::new(&mut file).finish(&mut df).unwrap();
}

fn save_immutable_part(df: &mut DataFrame, dir_path: &Path, step_id: u32) -> PolarsResult<PathBuf> {
    // 1. Define paths: Use sequential naming for the final file
    let final_name = format!("part_{:05}.parquet", step_id);
    let temp_name = format!("part_{:05}.parquet.tmp", step_id);
    
    let temp_path = dir_path.join(&temp_name);
    let final_path = dir_path.join(&final_name);

    // 2. Write to Temp
    {
        let mut temp_file = File::create(&temp_path)?;
        ParquetWriter::new(&mut temp_file).finish(df)?;
    } 

    // 3. Rename/Commit
    fs::rename(&temp_path, &final_path)?;
    
    Ok(final_path)
}
  *



    
pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let snapshot: SimulationSnapshot = serde_json::from_str(&json)?;
    Ok(snapshot)
}
    
pub fn generate_filename(behavior_name: &str, time: f64, step: usize) -> String {
    let clean_name = behavior_name.replace(" ", "_").to_lowercase();
    format!("{}_{:010.3}s_step_{:06}.json", clean_name, time, step)
}*/ 

