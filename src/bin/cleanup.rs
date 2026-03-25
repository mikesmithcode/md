use std::fs;
use std::io;
use std::path::Path;
use std::env;
use std::path::PathBuf;

fn clear_directory(path: PathBuf) -> io::Result<()> {
    

    if path.exists() && path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                fs::remove_file(entry_path)?;
            } else if entry_path.is_dir() {
                // Optional: Remove subdirectories recursively
                fs::remove_dir_all(entry_path)?;
            }
        }
        
    } else {
        println!("Directory not found, skipping");
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1{
        let path = Path::new("output").to_path_buf();
        clear_directory(path)?;
        println!("All output cleared");
    }else if args.len() == 2 {
        let folder = &args[1];
        // Paths relative to your workspace root
        let path = Path::new("output").join(folder);
        clear_directory(path)?;
        println!("All output in {:?} has been cleared", folder);
    }else{
        panic!("Supply a single folder as argument");
    }

    
    Ok(())
}
