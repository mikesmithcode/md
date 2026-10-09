import json
from pathlib import Path
import sys
import re
import shutil

    
def get_latest_file(src_dir, prefix="particles", extension="parquet"):
    """
    Finds the latest file in a directory matching <prefix>_<10-digit-step>.<extension>
    """
    src_path = Path(src_dir)
    
    # Check if the directory actually exists
    if not src_path.exists():
        print(f"Directory does not exist: {src_path.absolute()}")
        return None

    # Dynamic regex: matches e.g. particles_0001000100.parquet
    pattern = re.compile(rf"^{re.escape(prefix)}_(\d{{10}})\.{re.escape(extension)}$")
    
    max_step = -1
    latest_file = None
    
    # Use rglob if files might be nested, or glob for direct children
    for file_path in src_path.glob(f"{prefix}_*.{extension}"):
        match = pattern.match(file_path.name)
        if match:
            step_num = int(match.group(1))
            if step_num > max_step:
                max_step = step_num
                latest_file = file_path
        else:
            # Debug: see what failed to match if any files matched the glob
            # print(f"Filename didn't match regex: {file_path.name}")
            pass
            
    if latest_file is None:
        print(f"No matching files found in {src_path.absolute()} for pattern {prefix}_*_.{extension}")
        
    return latest_file

def save_dict_to_json(data_dict, filepath):
    """
    Takes a Python dictionary and writes it to a JSON file.
    """
    with open(filepath, 'w', encoding='utf-8') as f:
        json.dump(data_dict, f, indent=4, ensure_ascii=False)



def get_config(*args, **kwargs):
    """
    Parses the required input argument (e.g., 'silo' or 'silo_123') passed from the shell script.
    Splits at the first underscore to determine the target folder/config name.
    
    returns filepaths for the particles, objects, video and variables and returns the sim_settings
    """        
    input_name = sys.argv[1] 
    
    # Extract target name (everything before the first underscore, or the whole thing if no underscore)
    target_name = input_name.split('_', 1)[0]
    
    print(target_name)
    
    # Construct exact nested folder path:
    # output/<target_name>/<input_name>/particles/
    
    particles_dir = Path("output") / target_name / input_name / "particles"
    particles_dir.mkdir(parents=True, exist_ok=True)
    particles_filepath = particles_dir / "particles_0000000000.parquet"
    
    output_config_dir = Path("output") / target_name / input_name / "config"
    output_config_dir.mkdir(parents=True, exist_ok=True)
    
    objects_dir = Path("output") / target_name / input_name / "objects"
    objects_dir.mkdir(parents=True, exist_ok=True)
    objects_filepath = objects_dir / "objects_0000000000.parquet"
    
    video_dir = Path("output") / target_name / input_name / "video"
    video_dir.mkdir(parents=True, exist_ok=True)  
    
    
    # Define file paths (loads config based on the target name, e.g., 'silo.')
    config_dir = Path("input") / target_name
    config_dir.mkdir(parents=True, exist_ok=True)
    config_path = config_dir / "sim.json"
    
    print("\n===================================================================================\nget_config is copying model.json from input to your output folder\n===================================================================================\n")
    shutil.copy(str(config_dir / 'model.json'), str(output_config_dir / 'model_0000000000.json'))
    
    
    
    # Load configuration file
    with open(config_path, "r", encoding="utf-8") as f:
        config = json.load(f)
        
    return config, particles_filepath, objects_filepath
