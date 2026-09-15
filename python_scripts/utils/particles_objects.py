import numpy as np
import polars as pl
from typing import Union, Sequence, Tuple


from typing import Sequence, Tuple, Union
import numpy as np
import polars as pl

from typing import Dict, Optional, Sequence, Tuple, Union
import numpy as np
import polars as pl

# Default colour mapping matching previous values:
# 0: Main sphere (+q) -> White alpha=150
# 1: Charge sphere (+q) -> Magenta
# 2: Main sphere (-q) -> White alpha=150
# 3: Charge sphere (-q) -> Cyan
DEFAULT_PTYPE_COLOURS: Dict[int, Tuple[float, float, float, float]] = {
    0: (255.0, 255.0, 255.0, 150.0),
    1: (255.0, 0.0, 255.0, 255.0),
    2: (255.0, 255.0, 255.0, 150.0),
    3: (0.0, 255.0, 255.0, 255.0),
}


def generate_molecules(
    positions: Sequence[Tuple[float, float, float]],
    w: Union[Tuple[float, float, float], Sequence[Tuple[float, float, float]]] = (0.0, 0.0, 0.0),
    q_mag: Union[float, Sequence[float]] = 1e-9,
    v: Union[Tuple[float, float, float], Sequence[Tuple[float, float, float]]] = (0.0, 0.0, 0.0),
    rad: Union[float, Sequence[float]] = 0.005,
    d_r: Union[float, Sequence[float]] = 0.0,
    density: Union[float, Sequence[float]] = 1200,
    ptype: Union[int, Sequence[int]] = 0,
    mixed: bool = False,
    ptype_colours: Optional[Dict[int, Tuple[float, float, float, float]]] = None,
):
    """Yields a Polars DataFrame containing both the particle and its charge for each molecule.

    Sign logic:
      - If mixed=True and q_mag >= 0: Charges randomly pick positive or negative.
      - If mixed=False OR q_mag < 0: All charges get the sign of q_mag.

    Particle Types:
      - Positive molecule: Main ptype = 0, Charge ptype = 1
      - Negative molecule: Main ptype = 2, Charge ptype = 3
    """
    n_molecules = len(positions)

    # Merge user-defined colours with defaults
    colours = DEFAULT_PTYPE_COLOURS.copy()
    if ptype_colours is not None:
        colours.update(ptype_colours)

    def parse_scalar(param):
        if isinstance(param, (list, np.ndarray)) and len(param) == n_molecules:
            return param
        return [param] * n_molecules

    def parse_tuple(param):
        if isinstance(param, (list, np.ndarray)) and len(param) == n_molecules and isinstance(param[0], (list, tuple, np.ndarray)):
            return param
        return [param] * n_molecules

    #print('check', ptype)
    ws = parse_tuple(w)
    qs_mag = parse_scalar(q_mag)
    vels = parse_tuple(v)
    rads = parse_scalar(rad)
    d_rs = parse_scalar(d_r)
    densities = parse_scalar(density)
    ptypes = parse_scalar(ptype)

    phi = np.random.uniform(-np.pi, np.pi, size=n_molecules)

    # Determine signs per particle
    signs = np.zeros(n_molecules, dtype=int)
    for idx, q_val in enumerate(qs_mag):
        if mixed and q_val >= 0:
            signs[idx] = np.random.choice([1, -1])
        else:
            signs[idx] = 1 if q_val >= 0 else -1

    mol_id = 0
    particle_id = 0

    for i, pos_data in enumerate(positions):
        x, y, z = pos_data
        wx, wy, wz = ws[i]
        vx, vy, vz = vels[i]
        r = rads[i]
        dr = d_rs[i]
        dens = densities[i]
        q_val = qs_mag[i]

        sign = signs[i]
        actual_q = sign * abs(q_val)
        
        main_ptype = int(ptypes[i])
        charge_ptype = main_ptype + 1

        main_col = colours[main_ptype]
        charge_col = colours[charge_ptype]

        mass = (4.0 / 3.0) * np.pi * (r ** 3) * dens

        particle = {
            "t": 0.0,
            "id": int(particle_id),
            "molecule_id": int(mol_id),
            "ptype": int(main_ptype),
            "x": float(x), "y": float(y), "z": float(z),
            "rel_x": 0.0, "rel_y": 0.0, "rel_z": 0.0,
            "vx": float(vx), "vy": float(vy), "vz": float(vz),
            "wx": float(wx), "wy": float(wy), "wz": float(wz),
            "radius": float(r),
            "mass": float(mass),
            "charge": 0.0,
            "r": float(main_col[0]), "g": float(main_col[1]),
            "b": float(main_col[2]), "a": float(main_col[3])
        }
        particle_id += 1

        rel_pos = -r * dr

        charge = {
            "t": 0.0,
            "id": int(particle_id),
            "molecule_id": int(mol_id),
            "ptype": int(charge_ptype),
            "x": float(x + rel_pos * np.cos(phi[i])),
            "y": float(y),
            "z": float(z + rel_pos * np.sin(phi[i])),
            "rel_x": float(rel_pos * np.cos(phi[i])),
            "rel_y": 0.0,
            "rel_z": float(rel_pos * np.sin(phi[i])),
            "vx": float(vx), "vy": float(vy), "vz": float(vz),
            "wx": float(wx), "wy": float(wy), "wz": float(wz),
            "radius": float(0.2 * r),
            "mass": 0.0,
            "charge": float(actual_q),
            "r": float(charge_col[0]), "g": float(charge_col[1]),
            "b": float(charge_col[2]), "a": float(charge_col[3])
        }
        particle_id += 1
        mol_id += 1

        df = pl.concat([pl.DataFrame(particle), pl.DataFrame(charge)])

        df = df.with_columns(
            pl.col("ptype").cast(pl.UInt64),
            pl.col("id").cast(pl.UInt64),
            pl.col("molecule_id").cast(pl.UInt64)
        )

        yield df


def _base_object_df(id_val, time, velocity, omega, colour, visible, thickness):
    """Internal helper to generate the common columns for any object."""
    return {
        "t": [float(time)],
        "id": [int(id_val)],
        "vx": [float(velocity[0])], "vy": [float(velocity[1])], "vz": [float(velocity[2])],
        "wx": [float(omega[0])], "wy": [float(omega[1])], "wz": [float(omega[2])],
        "thickness": [float(thickness)],
        "r": [float(colour[0])], "g": [float(colour[1])], "b": [float(colour[2])], "a": [float(colour[3])],
        "visible": [bool(visible)],
    }

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
    
    data = _base_object_df(id_val, time, velocity, omega, colour, visible, thickness=0.0)
    data.update({
        "x1": [verts[0, 0]], "y1": [verts[0, 1]], "z1": [verts[0, 2]],
        "x2": [verts[1, 0]], "y2": [verts[1, 1]], "z2": [verts[1, 2]],
        "x3": [verts[2, 0]], "y3": [verts[2, 1]], "z3": [verts[2, 2]],
        "x4": [verts[3, 0]], "y4": [verts[3, 1]], "z4": [verts[3, 2]],
    })
    return pl.DataFrame(data)

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
    
    data = _base_object_df(id_val, time, velocity, omega, colour, visible, thickness=0.0)
    data.update({
        "x1": [verts[0, 0]], "y1": [verts[0, 1]], "z1": [verts[0, 2]],
        "x2": [verts[1, 0]], "y2": [verts[1, 1]], "z2": [verts[1, 2]],
        "x3": [verts[2, 0]], "y3": [verts[2, 1]], "z3": [verts[2, 2]],
        "x4": [np.nan], "y4": [np.nan], "z4": [np.nan],
    })
    return pl.DataFrame(data)

def create_line(
    vertices, 
    id_val=0, 
    thickness=0.001,
    time=0.0,
    velocity=(0.0, 0.0, 0.0), 
    omega=(0.0, 0.0, 0.0), 
    colour=(255, 255, 255, 255),
    visible=True
):
    verts = np.array(vertices, dtype=float)
    if verts.shape != (2, 3):
        raise ValueError("A line requires exactly 2 vertices of shape (2, 3).")
    
    data = _base_object_df(id_val, time, velocity, omega, colour, visible, thickness=thickness)
    data.update({
        "x1": [verts[0, 0]], "y1": [verts[0, 1]], "z1": [verts[0, 2]],
        "x2": [verts[1, 0]], "y2": [verts[1, 1]], "z2": [verts[1, 2]],
        "x3": [np.nan], "y3": [np.nan], "z3": [np.nan],
        "x4": [np.nan], "y4": [np.nan], "z4": [np.nan],
    })
    return pl.DataFrame(data)


def generate_particle_positions(n_particles, min_dist, **kwargs):
    min_bound = kwargs.get('min_bound', 0.005)
    max_bound = kwargs.get('max_bound', 0.015)
    h = kwargs.get('h', 0.02)

    positions = []
    max_attempts = 10000
    attempts = 0

    while len(positions) < n_particles and attempts < max_attempts:
        x = np.random.uniform(min_bound, max_bound)
        z = np.random.uniform(min_bound, max_bound)
        
        overlap = False
        for px, _, pz in positions:
            dist = np.sqrt((x - px)**2 + (z - pz)**2)
            if dist < min_dist:
                overlap = True
                break
                
        if not overlap:
            positions.append((x, h / 2.0, z))
            
        attempts += 1

    if len(positions) < n_particles:
        print(f"Warning: Only managed to place {len(positions)} out of {n_particles} non-overlapping particles.")

    return positions


def generate_particle_cube(nx, ny, nz, spacing, z_base, center_x, center_y):
    total_x = (nx - 1) * spacing
    total_y = (ny - 1) * spacing
    
    start_x = center_x - total_x / 2.0
    start_y = center_y - total_y / 2.0
    
    x = start_x + np.arange(nx) * spacing
    y = start_y + np.arange(ny) * spacing
    z = z_base + np.arange(nz) * spacing
    
    xx, yy, zz = np.meshgrid(x, y, z, indexing='ij')
    
    return np.column_stack((xx.ravel(), yy.ravel(), zz.ravel()))