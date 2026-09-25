from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))


import polars as pl
import matplotlib
import numpy as np
matplotlib.use('qtAgg')

from utils.file_io import get_config, save_dict_to_json
from utils.graphics import display
from utils.particles_objects import create_rectangle, create_line, generate_dipoles, generate_spheres, generate_particle_cube



"""Setup script designed to produce a bunch of regular particles with slightly varying radius. It also creates an initialising variables.json 
in the output / config directory."""


config, particles_filepath, objects_filepath, var_filepath = get_config()
#========================================================================
# Read simulation parameters to get simulation box dimensions
#========================================================================

box = config["sim_box_size"]
w = box[0]
d = box[1]
h = box[2]


#========================================================================
# This is for the dynamic balls. Drop a cube of particles under gravity
#========================================================================
r_ball = 0.001
dr_ball = 0.25 # variance in radius

pos_template = [(r_ball + r_ball * 2 * i, d / 2, h - r_ball) for i in range(11)]

spacing = 2.1*r_ball

dimensions = (round(w/spacing),round(d/spacing), round((0.85*h)/spacing))
start_pos = (spacing,spacing,0.15*h)
#cube of particles to drop
positions = generate_particle_cube(dimensions,spacing,start_pos,box, r_ball)
num_dynamic = positions.shape[0]
ptypes_dynamic = [0]*num_dynamic

#========================================================================
# This is for the static balls on the base of the cell
#========================================================================
base_spacing = 3.0*r_ball
base_dimensions = (round(w/base_spacing),round(d/base_spacing), 1)
start_base_pos = (2.0*r_ball,2.0*r_ball,r_ball)
static_particle_positions = generate_particle_cube(base_dimensions, base_spacing, start_base_pos,box, r_ball)
num_static = static_particle_positions.shape[0]
ptypes_static = [1]*num_static


#========================================================================
# Consolidate postions and radii of all balls
#========================================================================
positions = np.append(static_particle_positions, positions, axis=0)
rads = [r_ball - dr_ball * r_ball * np.random.uniform(1.0, 0.0) for _ in range(len(positions))]
ptypes = []
ptypes.extend(ptypes_static)
ptypes.extend( ptypes_dynamic)

# Define colour dictionary mapping ptype to (R, G, B, A)
ptype_colours = {
    0: (255.0, 255.0, 255.0, 150.0), # Positive main particle (White alpha=150)
    1: (255.0, 0.0, 255.0, 255.0),   # Positive charge (Magenta)
    2: (255.0, 255.0, 255.0, 150.0), # Negative main particle (White alpha=150)
    3: (0.0, 255.0, 255.0, 255.0),   # Negative charge (Cyan)
    4: (0.0, 255.0, 0.0, 150.0),    # Static particles
    5: (0.0, 255.0, 255.0, 255.0),    # Charges on static particles
}

#========================================================================
# Create particles
#========================================================================

molecules = list(generate_spheres(
    positions, 
    rad=rads, 
    ptype=ptypes,
    ptype_colours=ptype_colours
))

df = pl.concat(molecules)

#========================================================================
# Save to file
#========================================================================
df.write_parquet(particles_filepath)

#========================================================================
# Create and save variables.json initialisation in output/config
#========================================================================
variables = {
    "up": [0.0, 0.0, 1.0],
    "angle": 0.0
}

save_dict_to_json(variables, var_filepath)

#========================================================================
# Summarise
#========================================================================
print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype', 'r', 'g', 'b'].head(10))
display(df, box, objects_df=None)
