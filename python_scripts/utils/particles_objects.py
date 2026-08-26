import numpy as np
import polars as pl
from typing import Union, Sequence, Tuple


def generate_molecules(
    positions: Sequence[Tuple[float, float, float]], 
    w: Union[Tuple[float, float, float], Sequence[Tuple[float, float, float]]] = (0.0, 0.0, 0.0), 
    q: Union[float, Sequence[float]] = 1e-9, 
    v: Union[Tuple[float, float, float], Sequence[Tuple[float, float, float]]] = (0.0, 0.0, 0.0), 
    rad: Union[float, Sequence[float]] = 0.005, 
    d_r: Union[float, Sequence[float]] = 0.0, 
    density: Union[float, Sequence[float]] = 1200,
    particle_colour: Union[Tuple[float, float, float, float], Sequence[Tuple[float, float, float, float]]] = (255.0, 0.0, 0.0, 255.0),
    charge_colour: Union[Tuple[float, float, float, float], Sequence[Tuple[float, float, float, float]]] = (0.0, 0.0, 255.0, 255.0),
    offset=0.0001
):
    """
    A generator that yields a Polars DataFrame containing both the particle 
    and its charge for each molecule. Kwargs accept either a single value/tuple 
    or a list/array matching the length of positions.
    """
    n_molecules = len(positions)
    
    # Helper 1: For scalar/single values (e.g., rad, density, d_r, q)
    def parse_scalar(param):
        if isinstance(param, (list, np.ndarray)) and len(param) == n_molecules:
            return param
        return [param] * n_molecules

    # Helper 2: For tuple values (e.g., w, vel, particle_colour, charge_colour)
    def parse_tuple(param):
        if isinstance(param, (list, np.ndarray)) and len(param) == n_molecules and isinstance(param[0], (list, tuple, np.ndarray)):
            return param
        return [param] * n_molecules

    # Parse all keyword arguments
    ws = parse_tuple(w)
    qs = parse_scalar(q)
    vels = parse_tuple(v)
    rads = parse_scalar(rad)
    d_rs = parse_scalar(d_r)
    densities = parse_scalar(density)
    p_colours = parse_tuple(particle_colour)
    c_colours = parse_tuple(charge_colour)
    
    phi = np.random.uniform(-np.pi, np.pi, size=n_molecules)

    mol_id = 0
    particle_id = 0

    for i, pos_data in enumerate(positions):
        x, y, z = pos_data
        wx, wy, wz = ws[i]
        
        # Extract per-molecule values for this iteration
        vx, vy, vz = vels[i]
        r = rads[i]
        dr = d_rs[i]
        dens = densities[i]
        p_col = p_colours[i]
        c_col = c_colours[i]

        mass = (4.0 / 3.0) * np.pi * (r ** 3) * dens

        particle = {
            "t": [0.0],
            "id": [particle_id],
            "molecule_id": [mol_id],
            "ptype": [0],
            "x": [x], "y": [y], "z": [z],
            "rel_x": [0.0], "rel_y": [0.0], "rel_z": [0.0],
            "vx": [vx], "vy": [vy], "vz": [vz],
            "wx": [wx], "wy": [wy], "wz": [wz],
            "radius": [r],
            "mass": [mass],
            "charge": [qs[i]],
            "r": [p_col[0]], "g": [p_col[1]], 
            "b": [p_col[2]], "a": [p_col[3]]
        }
        particle_id += 1

        rel_pos = -r * dr
        
        charge = {
            "t": [0.0],
            "id": [particle_id],
            "molecule_id": [mol_id],
            "ptype": [1],
            "x": [x + rel_pos * np.cos(phi[i])], "y": [y+offset], "z": [z + rel_pos * np.sin(phi[i])],
            "rel_x": [rel_pos * np.cos(phi[i])], "rel_y": [0.0], "rel_z": [rel_pos * np.sin(phi[i])],
            "vx": [vx], "vy": [vy], "vz": [vz],
            "wx": [wx], "wy": [wy], "wz": [wz],
            "radius": [0.1 * r],
            "mass": [0.0],
            "charge": [0.0],
            "r": [c_col[0]], "g": [c_col[1]], 
            "b": [c_col[2]], "a": [c_col[3]]
        }
        particle_id += 1
        mol_id += 1

        # Combine particle and charge into a single DataFrame for this molecule
        df = pl.concat([pl.DataFrame(particle), pl.DataFrame(charge)])
        
        df = df.with_columns(
            pl.col("ptype").cast(pl.UInt64),
            pl.col("id").cast(pl.UInt64),
            pl.col("molecule_id").cast(pl.UInt64)
        )
        
        yield df


def create_rectangle(
    vertices, 
    id_val=0, 
    time=0.0,
    velocity=(0.0, 0.0, 0.0), 
    omega=(0.0, 0.0, 0.0), 
    colour=(255, 255, 255, 255),
    visible=True
):
    verts = np.array(vertices, dtype=float)
    if verts.shape != (4, 3):
        raise ValueError("A rectangle requires exactly 4 vertices of shape (4, 3).")
    
    return pl.DataFrame({
        "t": [float(time)],
        "id": [int(id_val)],
        "x1": [verts[0, 0]], "y1": [verts[0, 1]], "z1": [verts[0, 2]],
        "x2": [verts[1, 0]], "y2": [verts[1, 1]], "z2": [verts[1, 2]],
        "x3": [verts[2, 0]], "y3": [verts[2, 1]], "z3": [verts[2, 2]],
        "x4": [verts[3, 0]], "y4": [verts[3, 1]], "z4": [verts[3, 2]],
        "vx": [float(velocity[0])], "vy": [float(velocity[1])], "vz": [float(velocity[2])],
        "wx": [float(omega[0])], "wy": [float(omega[1])], "wz": [float(omega[2])],
        "r": [float(colour[0])], "g": [float(colour[1])], "b": [float(colour[2])], "a": [float(colour[3])],
        "visible": [bool(visible)],
    })

def create_triangle(
    vertices, 
    id_val=0, 
    time=0.0,
    velocity=(0.0, 0.0, 0.0), 
    omega=(0.0, 0.0, 0.0), 
    colour=(255, 255, 255, 255),
    visible=True
):
    verts = np.array(vertices, dtype=float)
    if verts.shape != (3, 3):
        raise ValueError("A triangle requires exactly 3 vertices of shape (3, 3).")
    
    return pl.DataFrame({
        "t": [float(time)],
        "id": [int(id_val)],
        "x1": [verts[0, 0]], "y1": [verts[0, 1]], "z1": [verts[0, 2]],
        "x2": [verts[1, 0]], "y2": [verts[1, 1]], "z2": [verts[1, 2]],
        "x3": [verts[2, 0]], "y3": [verts[2, 1]], "z3": [verts[2, 2]],
        "x4": [np.nan], "y4": [np.nan], "z4": [np.nan],  # NaN sentinels for triangle
        "vx": [float(velocity[0])], "vy": [float(velocity[1])], "vz": [float(velocity[2])],
        "wx": [float(omega[0])], "wy": [float(omega[1])], "wz": [float(omega[2])],
        "r": [float(colour[0])], "g": [float(colour[1])], "b": [float(colour[2])], "a": [float(colour[3])],
        "visible": [bool(visible)],
    })
