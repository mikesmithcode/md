import sys
import re
import shutil

from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))

from utils.file_io import get_config

def main():
    # Load configuration and get standardized destination paths via get_config()
    # Expects a command line argument like: python benchmark.py benchmark
    try:
        config, particles_filepath, objects_filepath = get_config()
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
    
    # Fallback to the hardcoded default if the custom one doesn't exist
    if not src_dir.exists():
        src_dir = Path("output/benchmarksetup/benchmarksetup/particles")

    if not src_dir.exists():
        print(f"[Error] Source directory not found: {src_dir}")
        return

    # Regex pattern to match files like particles_0000001230.parquet
    pattern = re.compile(r"^particles_(\d{10})\.parquet$")

    latest_file = None
    max_step = -1

    # Scan source directory for matching parquet files
    for file_path in src_dir.glob("particles_*.parquet"):
        match = pattern.match(file_path.name)
        if match:
            step_num = int(match.group(1))
            if step_num > max_step:
                max_step = step_num
                latest_file = file_path

    if latest_file is None:
        print(f"[Error] No valid particle parquet files found in {src_dir}")
        return

    print(f"Found latest snapshot: {latest_file.name} (Step {max_step})")

    # Ensure destination directory exists and copy the file using the path from get_config
    particles_filepath.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(latest_file, particles_filepath)
    
    print(f"Successfully copied to: {particles_filepath}")
    print(f"Loaded simulation config for '{target_name}' successfully.")

if __name__ == "__main__":
    main()
