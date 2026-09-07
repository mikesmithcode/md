import sys
from pathlib import Path
import polars as pl
import matplotlib.pyplot as plt
import numpy as np

# Add parent directory to path for utils
sys.path.append(str(Path(__file__).resolve().parent.parent))
from utils.file_io import get_config

def main():
    _, path_to_snapshots, _ = get_config()
    root = Path(__file__).parent.parent.parent
    
    parquet_path = root.joinpath(path_to_snapshots)
    folder = parquet_path.parent
    
    snapshot_files = sorted([f for f in folder.iterdir() if f.is_file() and f.suffix == '.parquet'])
    
    if not snapshot_files:
        print(f"No parquet files found in {folder}")
        return

    sqrt_dz_list = []
    speed_list = []
    z0 = None

    for idx, file_path in enumerate(snapshot_files):
        df = pl.read_parquet(file_path)
        
        subset = df.filter(pl.col("id") == 0)
        if len(subset) == 0:
            continue
        
        z = subset["z"][0]
        vx = subset["vx"][0] if "vx" in df.columns else 0.0
        vy = subset["vy"][0] if "vy" in df.columns else 0.0
        vz = subset["vz"][0] if "vz" in df.columns else 0.0
        
        if z0 is None:
            z0 = z
            
        delta_z = z0 - z
        speed = np.sqrt(vx**2 + vy**2 + vz**2)
        
        if delta_z >= 0:
            sqrt_dz_list.append(np.sqrt(delta_z))
            speed_list.append(speed)

    plt.figure(figsize=(8, 6))
    plt.plot(sqrt_dz_list, speed_list, label="Ball 0 (Simulation)", color="blue", linestyle="-", marker="o", markersize=2)

    plt.xlabel(r"$\sqrt{\Delta z}$ ($\sqrt{\text{m}}$)")
    plt.ylabel("Speed Magnitude ($v$)")
    plt.title("Rolling Ball")
    plt.legend()
    plt.grid(True)
    plt.show()

if __name__ == "__main__":
    main()