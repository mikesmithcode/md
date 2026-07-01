import json
from pathlib import Path
import numpy as np
import matplotlib.pyplot as plt
import os
import polars as pl

from utility import get_config, plot_circles_orientation, quaternion, display

input_path = Path(__file__).parent.parent.joinpath("input")
config_path = input_path.joinpath("abp.json")

config, snapshot_filepath = get_config(__file__)

box = config["sim_box_size"]

path_to_snapshots = Path("output/abp/snapshots/")
os.makedirs(path_to_snapshots, exist_ok=True)
filepath = path_to_snapshots.joinpath("snapshot_0000000000.parquet")

num_particles_target = 100 
n = int(np.sqrt(num_particles_target))

spacing_x = box[0] / (n + 1)
spacing_z = box[2] / (n + 1)

(qx,qy,qz,qw) = quaternion((0,1,0), 0)

base_particle = {
    "t": 0.0,
    "ptype": 0,
    "y": 0.0,
    "vx": 0.0,
    "vy": 0.0,
    "vz": 0.0,
    "qx": qx,
    "qy": qy,
    "qz": qz,
    "qw": qw,
    "wx": 0.0,
    "wy": 0.0,
    "wz": 0.0,
    "radius": 0.25,
    "mass": 2.35,
    "r": 255.0,
    "g": 0.0,
    "b": 0.0,
    "a": 255.0
}

particles_list = []
particle_id = 0

for i in range(n):
    for j in range(n):
        p = base_particle.copy()
        
        p["x"] = spacing_x + j * spacing_x
        p["z"] = spacing_z + i * spacing_z
        
        theta = np.random.uniform(0, 2 * np.pi)
        (p["qx"],p["qy"],p["qz"],p["qw"])=quaternion((0,1,0), theta)
        
        p["id"] = particle_id
        particle_id += 1
        particles_list.append(p)



df = pl.from_dicts(particles_list)
df = df.with_columns([
    pl.col("ptype").cast(pl.UInt64),
    pl.col("id").cast(pl.UInt64)
])

df.write_parquet(snapshot_filepath)
print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")

display(df, box)



