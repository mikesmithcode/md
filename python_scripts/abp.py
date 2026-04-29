"""Setup script for silo"""

from pathlib import Path
import numpy as np
import matplotlib.pyplot as plt
import os

import sys
print(f"DEBUG: Using Python at {sys.executable}")
import polars as pl

path_to_snapshots = "output/abp/snapshots/"

os.makedirs(path_to_snapshots, exist_ok=True)
root_path = Path(__file__).parent.parent
filepath = root_path.joinpath(path_to_snapshots,  "snapshot_0000000000.parquet")

base_particle = {
        "t": 0.0,
        "id": 0,
        "ptype": 0,
        "x" : 0.2,
        "y" : 0.2,
        "z" : 5.8,
        "vx" : 0.0,
        "vy" : 0.0,
        "vz" : 0.0,
        "phi_x" : 0.0,
        "phi_y" : 0.0,
        "phi_z" : 0.0,
        "wx" : 0.0,
        "wy" : 0.0,
        "wz" : 0.0,
        "radius" : 0.25,
        "mass" : 2.35,#density 1000kgm^-3
        "inertia" : 0.0,
        "r": 255.0,
        "g": 0.0,
        "b" : 0.0,
        "a" : 255.0
    }

particle = base_particle.copy()
d = 10.0/4.0


df = pl.DataFrame(particle)
#plt.figure()
#Create a square grid of particles
particle = base_particle.copy()
for i in range(4):
    for j in range(1):
        particle["x"] = base_particle["x"] + j*d
        particle["z"] = base_particle["z"] + i*d
        # 1. Generate a random angle between 0 and 2*pi
        theta = np.random.uniform(0, 2 * np.pi)
        particle["phi_x"] = np.cos(theta)
        particle["phi_z"] = np.sin(theta)
        plt.plot(particle["x"], particle["z"], 'go')
        theta =  np.random.uniform(low=0.0, high=2*np.pi)
        particle["vx"] = base_particle["x"] + np.cos(theta)
        particle["vz"] = base_particle["z"] + np.sin(theta)

        df_new = pl.DataFrame(particle)
        if not i==j==0:
            df=pl.concat([df, df_new])


    df=pl.concat([df, pl.DataFrame(particle)])

df = df.with_columns(pl.col("ptype").cast(pl.UInt64))
df = df.with_columns(pl.col("id").cast(pl.UInt64))


plt.show()
print("max", df.select(pl.max("ptype")))
print(df)

df.write_parquet(filepath)
        



