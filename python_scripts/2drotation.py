"""Setup script for coeff"""
import polars as pl
from pathlib import Path
import matplotlib
matplotlib.use('qtAgg')
import matplotlib.pyplot as plt

from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import generate_molecules, create_rectangle


config, particles_filepath, objects_filepath = get_config()
print(objects_filepath)
box = config["sim_box_size"]
print(box)

positions = [(0.02,0.0,0.02), (0.04,0.0,0.04)]
charges = [5E-9, -5E-9]

df = pl.concat(list(generate_molecules(positions, charges)))
df.write_parquet(particles_filepath)

w=box[0]
h=box[1]
z=0.005
rect = [(0.0,0.0,z),(0.0,h,z),(w,h,z),(w,0.0,z)]
# Create the rectangle DataFrame
rect_df = create_rectangle(
    vertices=rect, 
    id_val=0, 
    colour=(0, 255, 0, 255) # Optional styling color
)
rect_df.write_parquet(objects_filepath)



print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype','charge'].head())
display(df, box, objects_df=rect_df)
