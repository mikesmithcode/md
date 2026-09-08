
from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))


"""Setup script for coeff"""
import polars as pl
import matplotlib
matplotlib.use('qtAgg')
import numpy as np

from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import generate_molecules, create_rectangle, generate_particle_positions


config, particles_filepath, objects_filepath = get_config()
print(objects_filepath)
box = config["sim_box_size"]
print(box)

#Two moving particles and one static
# sim box is [0.02, 0.01, 0.02]
simbox = [0.02, 0.01, 0.02]
w,h,d = simbox

n_particles=20
radius = 0.001

positions = generate_particle_positions(n_particles, radius * 2.1, h=h, min_bound=radius, max_bound=w-radius)

# Generate slightly random velocities around zero
velocities = [
    (
        np.random.normal(0.0, 0.001), 
        0.0, 
        np.random.normal(0.0, 0.001)
    ) 
    for _ in range(n_particles)
]

q_magnitude = 0.15e-9

# Generate alternating charges for all particles
q = [q_magnitude for i in range(n_particles)]

d_r=0.8

molecules = list(generate_molecules(positions, v=velocities, d_r=d_r, rad=radius, q=q))


df = pl.concat(molecules)
df.write_parquet(particles_filepath)


print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype','charge'].head())
display(df, box)
