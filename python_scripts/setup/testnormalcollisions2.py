
from pathlib import Path
import sys

# Add the parent 'python_scripts' directory to sys.path
sys.path.append(str(Path(__file__).resolve().parent.parent))


"""Setup script for coeff"""
import polars as pl
import matplotlib
matplotlib.use('qtAgg')


from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import generate_molecules, create_rectangle


config, particles_filepath, objects_filepath = get_config()

box = config["sim_box_size"]


#Two moving particles and one static
positions = [(0.02,0.025,0.055),(0.02,0.025,0.03), (0.04,0.025,0.055), (0.04, 0.025, 0.005)]
velocities = [(0.0,0.0,-5.0),(0.0,0.0,0.0), (0.0,0.0,-5.0),(0.0,0.0,5.0)]
radii = [0.005, 0.005, 0.005, 0.005]
ptypes = [0, 0, 0, 0]

d_r = [0.5,0.5,0.5, 0.5]

molecules = list(generate_molecules(positions, v=velocities, d_r=d_r, rad=radii, ptype=ptypes))

df = pl.concat(molecules)
df.write_parquet(particles_filepath)


print(f"Successfully initialised {len(df)} particles")
print(df['ptype','charge'].head())
display(df, box)
