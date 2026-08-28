"""This is a utility script for looking at output"""

import polars as pl
from pathlib import Path
import matplotlib.pyplot as plt


script = "testnormalcollisions"

path_to_snapshots = Path("output/" + script + "/" + script + "/particles")

root = Path(__file__).parent.parent.parent
print(root)

folder = root.joinpath(path_to_snapshots)
files = sorted([f for f in folder.iterdir() if f.is_file()])

#0 is bouncing on surface
#1 is bouncing on a ball
vz0 = []
vz1 = []
z0 = []
z1 = []

z2 = 0.005
z_surf = 0.01
r1 = 0.0005 # dynamic
r2 = 0.005 # static

for filepath in files:
    # Check if file is empty or missing bytes
    if filepath.stat().st_size < 12:
        print(f"Skipping corrupted/empty file: {filepath}")
    else:
        df = pl.read_parquet(filepath)
    vz_values = df.filter(pl.col("ptype") == 0)["vz"].to_list()
    vz0.append(vz_values[0])
    vz1.append(vz_values[1])
    z_values = df.filter(pl.col("ptype") == 0)["z"].to_list()
    z0.append(z_values[0])
    z1.append(z_values[1])
    
#print(df[['id','vz','ptype','x','z']])

plt.figure(1)
plt.title("z vel")
plt.plot(vz0, "r-")
plt.plot(vz1, "b-")

plt.figure(2)
plt.title("z pos")
plt.plot(z0, "r-")#ball on ball
plt.plot(z1, "b-")#ball on surface
plt.show()
