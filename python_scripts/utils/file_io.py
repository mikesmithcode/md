import json
from pathlib import Path
import math
import numpy as np
import sys
import re

    
def get_latest_file(src_dir, prefix="particles", extension="parquet"):
    """
    Finds the latest file in a directory matching <prefix>_<10-digit-step>.<extension>
    e.g., particles_0000001500.parquet or variables_0000000500.json
    """
    # Dynamically build the regex pattern based on the prefix and extension
    pattern = re.compile(rf"^{re.escape(prefix)}_(\d{{10}})\.{re.escape(extension)}$")
    
    max_step = -1
    latest_file = None
    
    # Corrected glob usage to scan files in the directory
    for file_path in src_dir.glob(f"{prefix}_*.{extension}"):
        match = pattern.match(file_path.name)
        if match:
            step_num = int(match.group(1))
            if step_num > max_step:
                max_step = step_num
                latest_file = file_path
                
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
    
    objects_dir = Path("output") / target_name / input_name / "objects"
    objects_dir.mkdir(parents=True, exist_ok=True)
    objects_filepath = objects_dir / "objects_0000000000.parquet"
    
    video_dir = Path("output") / target_name / input_name / "video"
    video_dir.mkdir(parents=True, exist_ok=True)
    
    variables_dir = Path("output") / target_name / input_name / "config"
    variables_dir.mkdir(parents=True, exist_ok=True)
    variables_filepath = variables_dir / "variables_0000000000.json"
    
    
    
    # Define file paths (loads config based on the target name, e.g., 'silo.')
    config_dir = Path("input") / target_name 
    config_dir.mkdir(parents=True, exist_ok=True)
    config_path = config_dir / "sim_settings.json"
    
    
    
    # Load configuration file
    with open(config_path, "r", encoding="utf-8") as f:
        config = json.load(f)
        
    return config, particles_filepath, objects_filepath, variables_filepath