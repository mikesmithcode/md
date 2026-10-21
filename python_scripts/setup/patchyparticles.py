from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))

import numpy as np
from scipy.optimize import minimize
from utils.particles_objects import generate_charged_sphere, generate_charge_points, create_rectangle
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
    
    #========================================================================
    # Create patchy particles
    #========================================================================
    
        # Define colour dictionary mapping ptype to (R, G, B, A)
    ptype_colours = {
        0: (255.0, 255.0, 255.0, 150.0), # Positive main particle (White alpha=150)
        1: (255.0, 255.0, 255.0, 255.0),   # Positive charge (Magenta)
        2: (255.0, 255.0, 255.0, 150.0), # Negative main particle (White alpha=150)
        3: (0.0, 255.0, 255.0, 250.0),   # Negative charge (Cyan)
        4: (0.0, 255.0, 0.0, 150.0),    # Static particles
        5: (0.0, 255.0, 255.0, 250.0),    # Charges on static particles
    }
    
    positions = [(0.008,0.008,0.01), (0.012,0.012,0.025)]
    
    n_charges = 32 
    charges = generate_charge_points(n_charges)
    
    
    p = [generate_charged_sphere(positions[i], charges, mol_id=i, w=(0.0,10.0,0.0),ptype_colours=ptype_colours) for i in range(len(positions))]
    
    df = pl.concat(p)
    
    df.write_parquet(particles_filepath)
    
    #========================================================================
    # Create vibrating box
    #========================================================================
    
    base_height = 0.1*h
    
    

    cell = []

    #Base plate
    rect = [(0.0, 0.0, base_height), (0.0, d, base_height), (w, d, base_height), (w, 0.0, base_height)]
    left_rect = [(0.0, 0.0, base_height), (0.0, 0.0, h-2*base_height), (0.0, d, h-2*base_height), (0.0, d, base_height)]
    back_rect = [(0.0, d, base_height), (0.0, d, h-2*base_height), (w, d, h-2*base_height), (w, d, base_height)]
    right_rect = [(w, 0.0, base_height), (w, 0.0, h-2*base_height), (w, d, h-2*base_height), (w, d, base_height)]
    front_rect = [(0.0, 0.0, base_height), (0.0, 0.0, h-2*base_height), (w, 0.0, h-2*base_height), (w, 0.0, base_height)]
    
    base_df = create_rectangle(
        vertices=rect,
        id_val=0, 
        colour=(0, 255, 0, 255)
    )
    
    left_df = create_rectangle(
        vertices=left_rect, 
        id_val=0, 
        colour=(0, 255, 0, 100)
    )
    
    back_df = create_rectangle(
            vertices=back_rect, 
            id_val=0, 
            colour=(0, 255, 0, 100)
    )
    
    right_df = create_rectangle(
            vertices=right_rect, 
            id_val=0, 
            colour=(0, 255, 0, 100)
    )
    
    front_df = create_rectangle(
            vertices=front_rect, 
            id_val=0, 
            colour=(0, 255, 0, 0),
            visible=False
    )
        
    cell.append(base_df)
    cell.append(left_df)
    cell.append(back_df)
    cell.append(right_df)
    cell.append(front_df)
    
    objects = pl.concat(cell)
    objects.write_parquet(objects_filepath)
    
        
    print(df)
    
    display(df, box, objects_df=objects)

