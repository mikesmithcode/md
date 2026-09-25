import sys
import re
import shutil

from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))

from utils.file_io import get_config, get_latest_file

def main():
    # Load configuration and get standardized destination paths via get_config()
    # Expects a command line argument like: python benchmark.py benchmark
    try:
        config, particles_filepath, objects_filepath, var_dir = get_config()
    except IndexError:
        print("[Error] Please provide a run name argument (e.g., python benchmark.py benchmark)")
        sys.exit(1)

    # Determine source directory based on the input argument (e.g., looking in benchmarksetup)
    input_name = sys.argv[1]
    target_name = input_name.split('_', 1)[0]
    
    # Map to the corresponding setup folder structure
    source_target = f"{target_name}setup" if not target_name.endswith("setup") else target_name
    source_input = f"{input_name}setup" if not input_name.endswith("setup") else input_name
    
    src_dir = Path("output") / source_target / source_input / "particles"
    
    latest_particles_file = get_latest_file(src_dir, prefix="particles",extension=".parquet") 

    if latest_particles_file is None:
        print(f"[Error] No valid particle parquet files found in {src_dir}")
        return

    latest_particles_file = get_latest_file(src_dir, prefix="particles",extension=".parquet") 
    
    if latest_particles_file is None:
        print(f"[Error] No valid variables files found in {src_dir}")
        return
    
    latest_var_file = get_latest_file(var_dir, prefix="particles",extension=".parquet") 

    # Ensure destination directory exists and copy the file using the path from get_config
    particles_filepath.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(latest_particles_file, particles_filepath)
    
    print(f"Successfully copied to: {particles_filepath}")
    print(f"Loaded simulation config for '{target_name}' successfully.")

if __name__ == "__main__":
    main()
