import matplotlib.pyplot as plt
import matplotlib.patches as patches
from matplotlib.collections import PatchCollection
import json
from pathlib import Path
import os

NULL_ID = (1 << 63) - 1

def get_config(script_name):
    """call this function with get_config(__file__)"""
    input_path = Path(script_name).parent.parent.joinpath("input")
    name = Path(script_name).stem
    config_path = input_path.joinpath(f"{name}.json")

    with open(config_path, 'r') as f:
        config = json.load(f)
    
    path_to_snapshots = Path(f"output/{name}/snapshots/")
    os.makedirs(path_to_snapshots, exist_ok=True)
    snapshot_filepath = path_to_snapshots.joinpath("snapshot_0000000000.parquet")

    return config, snapshot_filepath

def plot_circles_orientation(df, ax):
    # Create a list of Circle patches using the actual radius from the data
    circles = [
        patches.Circle((x, z), radius=r) 
        for x, z, r in zip(df["x"], df["z"], df["radius"])
    ]

    # Create a collection for efficiency
    pc = PatchCollection(circles, edgecolors='black', facecolors='none', linewidths=1)
    ax.add_collection(pc)

    # Orientation vectors (quiver)
    # scale_units='xy' and angles='xy' ensure the arrows scale with the axes
    u = df["phi_x"] * df["radius"]
    w = df["phi_z"] * df["radius"]

    ax.quiver(
        df["x"], df["z"], u, w, 
        color='red', 
        scale=1,              # 1 data unit = 1 arrow unit
        scale_units='xy',     # Use the coordinate system for scale
        angles='xy',          # Arrows point correctly in the X-Z plane
        width=0.003,          # Thickness of the arrow shaft
        headwidth=3,
        pivot='tail'          # Ensures the arrow starts from the particle centre
    )

    return ax



