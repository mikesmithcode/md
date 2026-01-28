use std::fs;
use std::io;
use std::path::Path;

fn clear_directory(dir_path: &str) -> io::Result<()> {
    let path = Path::new(dir_path);

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
        println!("Cleared: {}", dir_path);
    } else {
        println!("Directory not found, skipping: {}", dir_path);
    }
    Ok(())
}

fn main() -> io::Result<()> {
    // Paths relative to your workspace root
    clear_directory("output/imgs")?;
    clear_directory("output/snapshots")?;
    
    println!("Clean-up complete.");
    Ok(())
}
