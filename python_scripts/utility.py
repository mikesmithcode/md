import matplotlib.pyplot as plt
import matplotlib.patches as patches
from matplotlib.collections import PatchCollection
import json
from pathlib import Path
import os


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

def plot_circles_orientation(df, ax, offset=(0, 10)):
    # Create a list of Circle patches using the actual radius from the data
    circles = [
        patches.Circle((x, z), radius=r) 
        for x, z, r in zip(df["x"] + df["rel_x"], df["z"] + df["rel_z"], df["radius"])
    ]

    # Create a collection for efficiency
    pc = PatchCollection(circles, edgecolors='black', facecolors='none', linewidths=1)
    ax.add_collection(pc)

    # Add ptype labels at the centre of each particle
    for x, z, ptype in zip(df["x"] + df["rel_x"], df["z"]+ df["rel_z"], df["ptype"]):
        ax.text(
            x, z, str(int(ptype)), 
            color='blue', 
            fontsize=8, 
            ha='center', 
            va='center',
            fontweight='bold'
        )
    
    if 'molecule_id' in df.columns:
        for x, z, m_type in zip(df["x"] + df["rel_x"], df["z"]+ df["rel_z"], df["molecule_id"]):
            ax.annotate(
                str(int(m_type)), 
                xy=(x, z),             # The position of the particle
                xytext=offset,        # Offset: 0 points right, 10 points up
                textcoords='offset points', 
                color='green', 
                fontsize=8, 
                ha='center', 
                va='bottom',
                fontweight='bold'
            )

    # Orientation vectors (quiver)
    u = df["phi_x"] * df["radius"]
    w = df["phi_z"] * df["radius"]

    ax.quiver(
        df["x"]+ df["rel_x"], df["z"]+ df["rel_z"], u, w, 
        color='red', 
        scale=1,              
        scale_units='xy',     
        angles='xy',          
        width=0.003,          
        headwidth=3,
        pivot='tail'          
    )

    return ax



