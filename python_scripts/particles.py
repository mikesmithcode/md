import numpy as np
import polars as pl

def generate_molecules(positions, vel=(0.0,0.0,0.0),w=(0.0,0.0,0.0), rad=0.005, q=1E-9, d_r=0.5, density=1200,
                          particle_colour=(255.0, 0.0, 0.0, 200.0),
                          charge_colour=(0.0, 255.0, 255.0, 240.0)):
    """
    A generator that yields a Polars DataFrame containing both the particle 
    and its charge for each molecule.
    
    positions = [(x1,y1,z1),(x2,y2,z2)]
    """
    mol_id = 0
    particle_id = 0
    
    vx, vy, vz = vel
    wx, wy, wz = w

    n_molecules = len(positions)
    
    phi = np.random.uniform(-np.pi, np.pi, size=n_molecules)

    for i,pos_data in enumerate(positions):
        x, y, z = pos_data
        mass = (4.0 / 3.0) * np.pi * (rad ** 3) * density

        particle = {
            "t": [0.0],
            "id": [particle_id],
            "molecule_id": [mol_id],
            "ptype": [0],
            "x": [x], "y": [y], "z": [z],
            "rel_x": [0.0], "rel_y": [0.0], "rel_z": [0.0],
            "vx": [vx], "vy": [vy], "vz": [vz],
            "wx": [wx], "wy": [wy], "wz": [wz],
            "radius": [rad],
            "mass": [mass],
            "charge": [0.0],
            "r": [particle_colour[0]], "g": [particle_colour[1]], 
            "b": [particle_colour[2]], "a": [particle_colour[3]]
        }
        particle_id += 1

        rel_pos = -rad *d_r
        
        
        charge = {
            "t": [0.0],
            "id": [particle_id],
            "molecule_id": [mol_id],
            "ptype": [1],
            "x": [x - 2*rad+rel_pos * np.cos(phi[i])], "y": [y], "z": [z + rel_pos * np.sin(phi[i])],
            "rel_x": [rel_pos * np.cos(phi[i])], "rel_y": [0.0], "rel_z": [rel_pos * np.sin(phi[i])],
            "vx": [vx], "vy": [vy], "vz": [vz],
            "wx": [wx], "wy": [wy], "wz": [wz],
            "radius": [0.1* rad],
            "mass": [0.0],
            "charge": [0.0],
            "r": [charge_colour[0]], "g": [charge_colour[1]], 
            "b": [charge_colour[2]], "a": [charge_colour[3]]
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