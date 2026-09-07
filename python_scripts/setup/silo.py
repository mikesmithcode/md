"""Setup script for silo"""

from pathlib import Path
import numpy as np
import matplotlib.pyplot as plt
import os
from python_scripts.utils.file_io import display, get_config_particlepath

import sys
print(f"DEBUG: Using Python at {sys.executable}")
import polars as pl



# Parse inputs and create folders etc.
config, particle_filepath = get_config_particlepath()

box = config["sim_box_size"]

base_particle = {
        "t": 0.0,
        "id": 0,
        "ptype": 0,
        "x" : 2.1,
        "y" : 0.0,
        "z" : 5.8,
        "vx" : 0.0,
        "vy" : 0.0,
        "vz" : 0.0,
        "qx" : 0.0,
        "qy" : 0.0,
        "qz" : 0.0,
        "qw" : 1.0,
        "wx" : 0.0,
        "wy" : 0.0,
        "wz" : 0.0,
        "radius" : 0.04,
        "mass" : 2.35,#density 1000kgm^-3
        "r": 255.0,
        "g": 0.0,
        "b" : 0.0,
        "a" : 255.0
    }

particle = base_particle.copy()
d = base_particle["radius"]*2.1


df = pl.DataFrame(particle)
id=0
#Create a square grid of particles
particle = base_particle.copy()
for i in range(70):
    for j in range(70):
        particle["id"]=id
        particle["x"] = base_particle["x"] + j*d
        particle["z"] = base_particle["z"] + i*d        

        df_new = pl.DataFrame(particle)
        if not i==j==0:
            df=pl.concat([df, df_new])
        id +=1
#Create a silo sides
particle = base_particle.copy()
angle_walls = 30.0 *np.pi/180.0
width_simbox = 10.0
num_wall_balls = int(1.5*width_simbox/(d))
x = np.linspace(0, 10.0,2*num_wall_balls)
z = np.abs((1/np.tan(angle_walls))*(x-5.0))
bottom_hopper = 2.0

for xval, zval in zip(x,z):
    particle["id"]=id
    particle["x"] = xval
    particle["z"] = zval
    if zval>= bottom_hopper:
        particle["r"]=0.0
        particle["b"]=255.0
        particle["g"]=0.0
        particle["ptype"]=1
        
    else:
        zval=bottom_hopper
        particle["z"] = bottom_hopper
        particle["r"]=0.0
        particle["b"]=0.0
        particle["g"]=255.0
        particle["ptype"]=2
    df=pl.concat([df, pl.DataFrame(particle)])
    id += 1

df = df.with_columns(pl.col("ptype").cast(pl.UInt64))
df = df.with_columns(pl.col("id").cast(pl.UInt64))

print("max", df.select(pl.max("ptype")))
print(df)

df.write_parquet(particle_filepath)
        

display(df, box)

