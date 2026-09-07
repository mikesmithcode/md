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

vz0 = []
vz1 = []
z0 = []
z1 = []
overlap0 = []
overlap1 = []

z_surf = 0.035  # Surface plane position for the second test case

for filepath in files:
    if filepath.stat().st_size < 12:
        print(f"Skipping corrupted/empty file: {filepath}")
        continue
        
    try:
        df = pl.read_parquet(filepath)
    except Exception as e:
        print(f"Skipping malformed or actively writing file {filepath.name}: {e}")
        continue
    
    df_dyn = df.filter(pl.col("ptype") == 0)
    df_stat = df.filter(pl.col("ptype") == 2)
    
    z_dyn = df_dyn["z"].to_list()
    vz_dyn = df_dyn["vz"].to_list()
    r_dyn = df_dyn["radius"].to_list()
    
    z_stat = df_stat["z"].to_list()
    r_stat = df_stat["radius"].to_list()
    
    if len(z_dyn) >= 2 and len(z_stat) >= 1:
        # --- Case 0: Ball-on-ball (dynamic particle 0 vs static ptype==2 ball) ---
        z0.append(z_dyn[0])
        vz0.append(vz_dyn[0])
        
        dist_ball = abs(z_dyn[0] - z_stat[0])
        ov_ball = (r_dyn[0] + r_stat[0]) - dist_ball # positive overlap is overlapping
        overlap0.append(ov_ball if ov_ball > 0 else float('nan'))
        
        # --- Case 1: Ball-on-surface (dynamic particle 1 vs surface plane) ---
        z1.append(z_dyn[1])
        vz1.append(vz_dyn[1])
        
        dist_surf = abs(z_dyn[1] - z_surf)
        ov_surf = r_dyn[1] - dist_surf # positive = overlapping
        overlap1.append(ov_surf if ov_surf > 0 else float('nan'))
    else:
        continue

fig, (ax1, ax2, ax3) = plt.subplots(3, 1, sharex=True, figsize=(8, 9))

ax1.set_title("z vel")
ax1.plot(vz0, "r-", label="vz0 (ball-on-ball)")
ax1.plot(vz1, "b-", label="vz1 (ball-on-surface)")
ax1.legend(loc='upper left')

ax2.set_title("z pos")
ax2.plot(z0, "r-", label="ball on ball")
ax2.plot(z1, "b-", label="ball on surface")
#ax2.legend()

ax3.set_title("overlap")
ax3.plot(overlap0, "rx", label="overlap (ball on ball)")
ax3.plot(overlap1, "bx", label="overlap (ball on surface)")
#ax3.legend()
print(overlap0)
print(overlap1)

plt.tight_layout()
plt.show()