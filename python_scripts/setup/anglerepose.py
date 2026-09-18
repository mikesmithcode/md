from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))

"""Setup script with mixed charges"""
import polars as pl
import matplotlib
import numpy as np
matplotlib.use('qtAgg')

from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import create_rectangle, create_line, generate_dipoles, generate_spheres, generate_particle_cube

config, particles_filepath, objects_filepath = get_config()

box = config["sim_box_size"]

# Sinusoidally vibrating rectangle surface
w = box[0]
d = box[1]
h = box[2]

z = 0.005
rect = [(0.0, 0.0, z), (0.0, d, z), (w, d, z), (w, 0.0, z)]

rect_df = create_rectangle(
    vertices=rect, 
    id_val=0, 
    colour=(0, 255, 0, 254)
)
#rect_df.write_parquet(objects_filepath)


r_ball = 0.005
dr_ball = 0.25 # variance in radius

#Add an up_line
up_line_vertices = [(0.0,d, -2.5*r_ball),(w,d, -5*r_ball)]


line = create_line(up_line_vertices, thickness=0.002, colour=(0,255,0,255))
line.write_parquet(objects_filepath)





pos_template = [(r_ball + r_ball * 2 * i, d / 2, h - r_ball) for i in range(11)]

spacing = 3.0*r_ball

dimensions = (round(w/spacing),round(d/spacing), round((0.85*h)/spacing))
start_pos = (spacing,spacing,0.15*h)

base_dimensions = (round(w/spacing),round(d/spacing), 1)
start_base_pos = (2.0*r_ball,2.0*r_ball,r_ball)
#grid of static particles at the bottom
static_particle_positions = generate_particle_cube(base_dimensions, spacing, start_base_pos,box, r_ball)
#cube of particles to drop
positions = generate_particle_cube(dimensions,spacing,start_pos,box, r_ball)

num_dynamic = positions.shape[0]
num_static = static_particle_positions.shape[0]

positions = np.append(static_particle_positions, positions, axis=0)

print('positions', positions)


rads = [r_ball - dr_ball * r_ball * np.random.uniform(1.0, 0.0) for _ in range(len(positions))]

d_r = 0.0 # fractional position of charge
q_mag = 0.0e-9   # Charge magnitude
mixed = True   # Enable mixed positive/negative charge generation

ptypes_static = [4]*num_static

bool_list = np.random.choice([True, False], size=num_dynamic).tolist()
ptypes_dynamic = [0 if val else 2 for val in bool_list]

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

molecules = list(generate_dipoles(
    positions, 
    rad=rads, 
    ptype=ptypes,
    d_r=d_r, 
    q_mag=q_mag, 
    mixed=mixed,
    ptype_colours=ptype_colours,
))

df = pl.concat(molecules)

# remove the charges
#df = df.filter(pl.col("ptype") % 2 == 0)

df.write_parquet(particles_filepath)

print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype', 'charge', 'r', 'g', 'b'].head(10))
display(df, box, objects_df=None)
