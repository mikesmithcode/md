use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};

fn backup_folder(src: PathBuf) -> String {
    let now: DateTime<Local> = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
    
    let folder_name = src.file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source path")).expect("Problem with folder name")
        .to_string_lossy();

    let dest_name = format!("{}_{}", folder_name, timestamp);
    let dest = src.with_file_name(&dest_name);

    copy_dir_recursive(&src, &dest).expect("Err copying files");
    dest_name
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2{
        let folder = &args[1];
        // Paths relative to your workspace root
        let path = Path::new("output").join(folder);
        let dest_name = backup_folder(path);
        println!("Simulation output in {:?} has been copied to {:?}", folder, dest_name);
    }else{
        panic!("Supply a single folder as argument");
    }

    
    Ok(())
}
