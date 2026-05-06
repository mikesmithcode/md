import numpy as np
import matplotlib.pyplot as plt
import polars as pl

from utility import get_config, plot_circles_orientation



config, snapshot_filepath = get_config(__file__)

box_x = config["sim_box_size"][0]
box_z = config["sim_box_size"][2]



num_particles_target = 100 
n = int(np.sqrt(num_particles_target))

spacing_x = box_x / (n + 1)
spacing_z = box_z / (n + 1)

#assume density of 2000kgm^-3
# Assume solid sphere.
base_particle = {
    "t": 0.0,
    "ptype": 0,
    "y": 0.0,
    "vx": 0.0,
    "vy": 0.0,
    "vz": 0.0,
    "phi_y": 0.0,
    "wx": 0.0,
    "wy": 0.0,
    "wz": 0.0,
    "radius": 0.005,
    "mass": 0.001047166666666667,
    "inertia": 1.047166666666667e-08,
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
        p["phi_x"] = np.cos(theta)
        p["phi_z"] = np.sin(theta)
        
        p["id"] = particle_id
        particle_id += 1
        particles_list.append(p)


df = pl.from_dicts(particles_list)
df = df.with_columns([
    pl.col("ptype").cast(pl.UInt64),
    pl.col("id").cast(pl.UInt64)
])


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
