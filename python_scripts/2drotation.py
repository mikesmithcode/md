"""Setup script for coeff"""
import polars as pl

from pathlib import Path
import matplotlib
# Use Qt6Agg to leverage your newly installed PyQt6
matplotlib.use('qtAgg')
import matplotlib.pyplot as plt
from utility import get_config, display
from particles import generate_molecules


config, particles_filepath, objects_filepath = get_config()
box = config["sim_box_size"]
print(box)

positions = [(0.02,0.0,0.02), (0.04,0.0,0.04)]

df = pl.concat(list(generate_molecules(positions)))
df.write_parquet(particles_filepath)

print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype','charge'].head())
display(df, box)
