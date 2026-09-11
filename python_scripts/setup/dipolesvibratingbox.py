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
from utils.particles_objects import create_rectangle, generate_molecules

config, particles_filepath, objects_filepath = get_config()
print(objects_filepath)
box = config["sim_box_size"]
print(box)

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
rect_df.write_parquet(objects_filepath)

r_ball = 0.0025
dr_ball = 0.25 # variance in radius

pos_template = [(r_ball + r_ball * 2 * i, d / 2, h - r_ball) for i in range(11)]
positions = []
for j in range(8):
    if j % 2 == 0:
        pos1 = [(pos[0] + r_ball, pos[1], h - (2 * j + 1) * r_ball) for pos in pos_template]
    if j % 2 == 1:
        pos1 = [(pos[0], pos[1], h - (2 * j + 1) * r_ball) for pos in pos_template]
    positions.extend(pos1)

rads = [r_ball - dr_ball * r_ball * np.random.uniform(1.0, 0.0) for _ in range(len(positions))]

d_r = 0.75 # fractional position of charge
q_mag = 1e-9   # Charge magnitude
mixed = True   # Enable mixed positive/negative charge generation

# Define colour dictionary mapping ptype to (R, G, B, A)
ptype_colours = {
    0: (255.0, 255.0, 255.0, 150.0), # Positive main particle (White alpha=150)
    1: (255.0, 0.0, 255.0, 255.0),   # Positive charge (Magenta)
    2: (255.0, 255.0, 255.0, 150.0), # Negative main particle (White alpha=150)
    3: (0.0, 255.0, 255.0, 255.0),   # Negative charge (Cyan)
}

molecules = list(generate_molecules(
    positions, 
    rad=rads, 
    d_r=d_r, 
    q_mag=q_mag, 
    mixed=mixed,
    ptype_colours=ptype_colours,
))

df = pl.concat(molecules)
df.write_parquet(particles_filepath)

print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype', 'charge', 'r', 'g', 'b'].head(10))
display(df, box, objects_df=rect_df)
