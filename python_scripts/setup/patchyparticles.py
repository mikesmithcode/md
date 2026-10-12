from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))

import numpy as np
from scipy.optimize import minimize
from utils.particles_objects import generate_charged_sphere, generate_charge_points
from utils.graphics import display
from utils.file_io import get_config
import polars as pl



if __name__ == '__main__':
    config, particles_filepath, objects_filepath = get_config()
    #========================================================================
    # Read simulation parameters to get simulation box dimensions
    #========================================================================

    box = config["sim_box_size"]
    w = box[0]
    d = box[1]
    h = box[2]
    
    print('box', box)
    
        # Define colour dictionary mapping ptype to (R, G, B, A)
    ptype_colours = {
        0: (255.0, 255.0, 255.0, 250.0), # Positive main particle (White alpha=150)
        1: (255.0, 0.0, 255.0, 250.0),   # Positive charge (Magenta)
        2: (255.0, 255.0, 255.0, 150.0), # Negative main particle (White alpha=150)
        3: (0.0, 255.0, 255.0, 250.0),   # Negative charge (Cyan)
        4: (0.0, 255.0, 0.0, 150.0),    # Static particles
        5: (0.0, 255.0, 255.0, 250.0),    # Charges on static particles
    }
    
    positions = [(0.008,0.0,0.01),(0.012,0.0,0.03)]
    
    n_charges = 32 
    charges = generate_charge_points(n_charges)
    
    
    p = [generate_charged_sphere(positions[i], charges) for i in range(2)]
    
    df = pl.concat(p)
    print(df)
    
    display(df, box, objects_df=None)

