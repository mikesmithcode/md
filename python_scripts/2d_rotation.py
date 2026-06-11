"""Setup script for coeff"""



import polars as pl

from pathlib import Path
import matplotlib
# Use Qt6Agg to leverage your newly installed PyQt6
matplotlib.use('qtAgg')
import matplotlib.pyplot as plt
import os


from utility import get_config, plot_circles_orientation


config, snapshot_filepath = get_config(__file__)
box_x = config["sim_box_size"][0]
box_z = config["sim_box_size"][2]

id=0

#'x','y','z' specifies the molecules centre of mass. Global position = eg x + rel_x

base_particle = {
        "t": 0.0,
        "id": id,
        "molecule_id": 0, 
        "ptype": 0,
        "x" : 0.025,
        "y" : 0.005,
        "z" : 0.005,
        "rel_x": 0.0,
        "rel_y": 0.0,
        "rel_z": 0.0,
        "vx" : 0.0,
        "vy" : 0.0,
        "vz" : 0.0,
        "phi_x" : 1.0,
        "phi_y" : 0.0,
        "phi_z" : 0.0,
        "phi_w" : 0.0,
        "wx" : 0.0,
        "wy" : 0.0,
        "wz" : 0.0,
        "radius" : 0.005,
        "mass" : 5.2E-4,#density 1000kgm^-3
        "inertia" : 0.0,
        "charge" : 0.0,
        "r": 255.0,
        "g": 0.0,
        "b" : 0.0,
        "a" : 150.0
    }


#particles
particle = base_particle.copy()
particle2 = base_particle.copy()
particle["z"] = 0.045
particle["wy"] = 1.0
particle["molecule_id"] = 0
#particle["x"] += 0.0005
id += 1
particle2["id"] = id
particle2["molecule_id"] = 1
particle2["ptype"] = 1

#charges
charge = particle.copy()
id += 1
charge["id"] = id
charge["ptype"] = 2
charge["molecule_id"]= 0
charge["radius"]=0.1*particle["radius"]
charge["rel_x"] = particle["radius"]*0.5*particle["phi_x"]
charge["rel_y"] = 0.0
charge["rel_z"] = 0.0
charge["r"] = 0.0
charge["g"] = 255.0
charge["b"] = 0.0
charge["a"] = 255.0

charge2 = particle2.copy()
id += 1
charge2["id"] = id
charge2["ptype"] = 3
charge2["molecule_id"]= 1
charge2["radius"]=0.1*particle2["radius"]
charge2["rel_x"] = particle2["radius"]*0.5*particle2["phi_x"]
charge2["rel_y"] = particle2["radius"]*0.5*particle2["phi_y"]
charge2["rel_z"] = particle2["radius"]*0.5*particle2["phi_z"]
charge2["r"] = 0.0
charge2["g"] = 255.0
charge2["b"] = 0.0
charge2["a"] = 255.0


df = pl.DataFrame(particle)
df_2 = pl.DataFrame(particle2)
df_3 = pl.DataFrame(charge)
df_4 = pl.DataFrame(charge2)
df=pl.concat([df, df_2])
df=pl.concat([df, df_3])
df=pl.concat([df, df_4])

df = df.with_columns(pl.col("ptype").cast(pl.UInt64))
df = df.with_columns(pl.col("id").cast(pl.UInt64))
df = df.with_columns(pl.col("molecule_id").cast(pl.UInt64))

df.write_parquet(snapshot_filepath)
print(df)

fig, ax = plt.subplots(figsize=(8, 8))

ax = plot_circles_orientation(df, ax)

# Set axis limits and ensure aspect ratio is 1:1 so circles aren't ellipses
ax.set_xlim(0, box_x)
ax.set_ylim(0, box_z)
ax.set_aspect('equal')

plt.title(f"SI Units Initialisation: {len(df)} Particles (True Scale)")
plt.xlabel("X (m)")
plt.ylabel("Z (m)")
plt.grid(True, linestyle=':', alpha=0.6)
plt.show()


df.write_parquet(snapshot_filepath)
print(f"Successfully initialised {len(df)} particles for a {box_x}x{box_z} box.")

