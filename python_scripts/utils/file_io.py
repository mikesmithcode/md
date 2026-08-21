import json
from pathlib import Path
import math
import numpy as np
import sys


def get_config(*args, **kwargs):
    """
    Parses the required input argument (e.g., 'silo' or 'silo_123') passed from the shell script.
    Splits at the first underscore to determine the target folder/config name.
    """        
    input_name = sys.argv[1] # e.g., 'silo' or 'silo_123'
    
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
    
    
    # Define file paths (loads config based on the target name, e.g., 'silo.')
    config_dir = Path("input") / target_name 
    config_dir.mkdir(parents=True, exist_ok=True)
    config_path = config_dir / "sim_settings.json"
    
    
    # Load configuration file
    with open(config_path, "r", encoding="utf-8") as f:
        config = json.load(f)
        
    return config, particles_filepath, objects_filepath

