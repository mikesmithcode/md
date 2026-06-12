"""Setup script for ball_surface"""

import polars as pl
from pathlib import Path
import matplotlib.pyplot as plt
import os
import sys
from utility import display, get_config


name = sys.argv[1]

path = "output/" + name
path_to_snapshots = path + "/snapshots"
config, snapshot_filepath = get_config(__file__)
box = config["sim_box_size"]

os.makedirs(path_to_snapshots, exist_ok=True)

root_path = Path.cwd()
filepath = root_path.joinpath(path_to_snapshots,  "snapshot_0000000000.parquet")

base_particle = {
        "t": 0.0,
        "id": 0,
        "ptype": 0,
        "x" : 0.022,
        "y" : 0.005,
        "z" : 0.005,
        "vx" : 0.0,
        "vy" : 0.0,
        "vz" : 0.0,
        "wx" : 0.0,
        "wy" : 0.0,
        "wz" : 0.0,
        "radius" : 0.005,
        "mass" : 5.2E-4,#density 1000kgm^-3
        "inertia": 0.0,
        "r": 255.0,
        "g": 0.0,
        "b" : 0.0,
        "a" : 255.0,
    }

particle2 = base_particle.copy()
particle2["z"] = 0.045
particle2["id"] = 1
particle2["ptype"] = 0



df = pl.DataFrame(particle2)

df = df.with_columns(
    pl.col("id").cast(pl.UInt64),
    pl.col("ptype").cast(pl.UInt64)
)


df.write_parquet(filepath)
print(df)

display(df, box)