import matplotlib.pyplot as plt
import matplotlib.patches as patches
from matplotlib.collections import PatchCollection
import json
from pathlib import Path
import os
import math
import numpy as np

def quaternion(axis: tuple[float, float, float], theta: float) -> tuple[float, float, float, float]:
    """
    Converts a 3D axis and a rotation angle (in radians) into a normalised unit quaternion.
    
    Returns:
        tuple: (x, y, z, w)
    """
    # 1. Ensure the input axis vector is normalised to unit length
    ax, ay, az = axis
    magnitude = math.sqrt(ax**2 + ay**2 + az**2)
    
    # Handle the edge case of a zero vector passed as an axis
    if magnitude == 0:
        return (0.0, 0.0, 0.0, 1.0) # Return the identity quaternion
        
    ux = ax / magnitude
    uy = ay / magnitude
    uz = az / magnitude

    # 2. Calculate half-angle trigonometric values
    half_theta = theta / 2.0
    sin_half = math.sin(half_theta)
    cos_half = math.cos(half_theta)

    # 3. Formulate the quaternion components
    x = ux * sin_half
    y = uy * sin_half
    z = uz * sin_half
    w = cos_half

    return (x, y, z, w)

def theta_from_quaternion_xz(df):
    """
    Calculates (cos(theta), sin(theta)) from quaternion arrays or scalars.
    Assumes a rotation around the Y-axis acting on local forward vector [1, 0, 0].
    """
    # Force inputs into numpy arrays to support both scalars and series safely
    qy = np.array(df['qy'])
    qw = np.array(df['qw'])
    
    cos_theta = qw**2 - qy**2
    sin_theta = -2.0 * qy * qw
    
    return (cos_theta, sin_theta)

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
        for x, z, r in zip(df["x"], df["z"], df["radius"])
    ]

    # Create a collection for efficiency
    pc = PatchCollection(circles, edgecolors='black', facecolors='none', linewidths=1)
    ax.add_collection(pc)

    # Add ptype labels at the centre of each particle
    for x, z, ptype in zip(df["x"], df["z"], df["ptype"]):
        ax.text(
            x, z, str(int(ptype)), 
            color='blue', 
            fontsize=8, 
            ha='center', 
            va='center',
            fontweight='bold'
        )
    
    if 'molecule_id' in df.columns:
        for x, z, m_type in zip(df["x"], df["z"], df["molecule_id"]):
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
    if "qy" in df.columns:
        (cos_theta, sin_theta) = theta_from_quaternion_xz(df)
        
        u = cos_theta * df["radius"]
        w = sin_theta * df["radius"]

        ax.quiver(
            df["x"], df["z"], u, w, 
            color='red', 
            scale=1,              
            scale_units='xy',     
            angles='xy',          
            width=0.003,          
            headwidth=3,
            pivot='tail'          
        )

        return ax

def display(df, box):
    fig, ax = plt.subplots(figsize=(8, 8))
    
    plot_circles_orientation(df, ax)

    # Set axis limits and ensure aspect ratio is 1:1 so circles aren't ellipses
    ax.set_xlim(0, box[0])
    ax.set_ylim(0, box[2])
    ax.set_aspect('equal')

    plt.title(f"SI Units Initialisation: {len(df)} Particles (True Scale)")
    plt.xlabel("X (m)")
    plt.ylabel("Z (m)")
    plt.grid(True, linestyle=':', alpha=0.6)
    plt.show()


   