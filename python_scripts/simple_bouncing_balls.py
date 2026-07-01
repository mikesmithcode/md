"""Setup script for simple_bouncing_balls"""

import polars as pl

from pathlib import Path

import matplotlib.pyplot as plt
import os



path = "output/simple_bouncing_balls"
path_to_snapshots = path + "/snapshots"


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
        "qx" : 0.0,
        "qy" : 0.0,
        "qz" : 0.0,
        "wx" : 0.0,
        "wy" : 0.0,
        "wz" : 0.0,
        "radius" : 0.005,
        "mass" : 5.2E-4,#density 1000kgm^-3
        "r": 255.0,
        "g": 0.0,
        "b" : 0.0,
        "a" : 255.0
    }

particle2 = base_particle.copy()
particle2["z"] = 0.045
particle2["id"] = 1
particle2["ptype"] = 0
df2 = pl.DataFrame(particle2)

particle = base_particle.copy()
particle["ptype"] = 1
particle["radius"] = 0.0005
particle["x"] = 0.0
df = pl.DataFrame(particle)
df = pl.concat([df, df2])
for i in range(49):
    particle["x"] = particle["x"] + 2.0*particle["radius"]
    df_2 = pl.DataFrame(particle.copy())
    df=pl.concat([df, df_2])

df = df.with_columns(pl.col("ptype").cast(pl.UInt64))
df = df.with_columns(pl.col("id").cast(pl.UInt64))

df.write_parquet(filepath)
print(df)

# Scatter plot of two columns
plt.figure()
plt.scatter(df["x"], df["z"])
plt.show()
print("max", df.select(pl.max("ptype")))

