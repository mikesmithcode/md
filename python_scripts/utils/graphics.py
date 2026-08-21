import matplotlib.pyplot as plt
import matplotlib.patches as patches
from matplotlib.collections import PatchCollection
from .quaternions import theta_from_quaternion_xz
import numpy as np

def plot_circles_orientation(df, ax, offset=(0, 10), colours=None):
    if df.is_empty():
        return ax

    if colours is None:
        colours = {}

    x_vals = df["x"].to_numpy()
    z_vals = df["z"].to_numpy()
    radii = df["radius"].to_numpy()
    ptypes = df["ptype"].to_numpy()

    # Extract RGBA columns if they exist, otherwise default to a light gray
    has_color = all(col in df.columns for col in ["r", "g", "b", "a"])
    if has_color:
        face_colors = np.column_stack([
            df["r"].to_numpy() / 255.0,
            df["g"].to_numpy() / 255.0,
            df["b"].to_numpy() / 255.0,
            df["a"].to_numpy() / 255.0
        ])
    else:
        face_colors = 'none'

    # 1. Filled Circle Patches (zorder=1)
    circles = [
        patches.Circle((x, z), radius=r) 
        for x, z, r in zip(x_vals, z_vals, radii)
    ]
    pc = PatchCollection(
        circles, 
        edgecolors='black', 
        facecolors=face_colors, 
        linewidths=1, 
        zorder=1
    )
    ax.add_collection(pc)

    # 2. Orientation Vectors / Quiver (zorder=2)
    if "qy" in df.columns:
        cos_theta, sin_theta = theta_from_quaternion_xz(df)
        u = cos_theta * radii
        w = sin_theta * radii

        ax.quiver(
            x_vals, z_vals, u, w, 
            color='red', 
            scale=1,              
            scale_units='xy',     
            angles='xy',          
            width=0.003,          
            headwidth=3,
            pivot='tail',
            zorder=2
        )

    # 3. Particle Type Labels (zorder=3 - Always on top)
    for x, z, ptype in zip(x_vals, z_vals, ptypes):
        ax.text(
            x, z, str(int(ptype)), 
            color=colours.get(int(ptype), "black"), 
            fontsize=8, 
            ha='center', 
            va='center',
            fontweight='bold',
            zorder=3
        )
    
    # 4. Molecule ID Annotations (zorder=3 - Always on top)
    if 'molecule_id' in df.columns:
        m_types = df["molecule_id"].to_numpy()
        for x, z, m_type in zip(x_vals, z_vals, m_types):
            ax.annotate(
                str(int(m_type)), 
                xy=(x, z),
                xytext=offset,
                textcoords='offset points', 
                color='green', 
                fontsize=8, 
                ha='center', 
                va='bottom',
                fontweight='bold',
                zorder=3
            )

    return ax


def plot_objects(df, ax):
    """
    Takes an objects DataFrame and plots filled rectangles and triangles 
    onto the provided Matplotlib axis behind the text layers.
    """
    if df.is_empty():
        return ax

    patches_list = []
    face_colors = []
    edge_colors = []

    for row in df.iter_rows(named=True):
        verts = [
            [row["x1"], row["z1"]],
            [row["x2"], row["z2"]],
            [row["x3"], row["z3"]]
        ]
        
        x4 = row["x4"]
        if x4 is not None and not np.isnan(x4):
            verts.append([x4, row["z4"]])

        poly = patches.Polygon(verts, closed=True)
        patches_list.append(poly)

        # Normalize RGBA color from [0, 255] to [0, 1]
        rgba = (
            row["r"] / 255.0,
            row["g"] / 255.0,
            row["b"] / 255.0,
            row["a"] / 255.0
        )
        face_colors.append(rgba)
        edge_colors.append('black')

        # Object ID text positioned at the centroid (zorder=3)
        centroid_x = np.mean([v[0] for v in verts])
        centroid_z = np.mean([v[1] for v in verts])
        ax.text(
            centroid_x, centroid_z, str(int(row["id"])),
            color='white', fontsize=7, ha='center', va='center', fontweight='bold',
            zorder=3
        )

    # Patch collection for structural objects rendered in the background (zorder=1)
    pc = PatchCollection(
        patches_list, 
        facecolors=face_colors, 
        edgecolors=edge_colors, 
        linewidths=1,
        zorder=1
    )
    ax.add_collection(pc)

    return ax


def display(df, box, objects_df=None):
    fig, ax = plt.subplots(figsize=(8, 8))
    
    # 1. Plot particles
    plot_circles_orientation(df, ax)

    # 2. Plot structural objects (rectangles/triangles) if provided
    num_objects = 0
    if objects_df is not None and not objects_df.is_empty():
        plot_objects(objects_df, ax)
        num_objects = len(objects_df)

    # Set axis limits and ensure aspect ratio is 1:1 so shapes aren't distorted
    ax.set_xlim(0, box[0])
    ax.set_ylim(0, box[2])
    ax.set_aspect('equal')

    plt.title(f"SI Units Initialisation: {len(df)} Particles, {num_objects} Objects (True Scale)")
    plt.xlabel("X (m)")
    plt.ylabel("Z (m)")
    plt.grid(True, linestyle=':', alpha=0.6)
    plt.show()

   