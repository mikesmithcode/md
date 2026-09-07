import sys
from pathlib import Path
import polars as pl
import matplotlib.pyplot as plt

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

    times = []
    v_data = {0: [], 2: [], 4: [], 6: []}

    for idx, file_path in enumerate(snapshot_files):
        df = pl.read_parquet(file_path)
        
        subset = df.filter(pl.col("id").is_in([0, 2, 4, 6]))
        
        t = subset["time"][0] if "time" in df.columns else idx
        times.append(t)
        
        for pid in [0, 2, 4, 6]:
            p_row = subset.filter(pl.col("id") == pid)
            if len(p_row) > 0:
                vz = p_row["vz"][0]
                v_data[pid].append(vz)
            else:
                v_data[pid].append(float('nan'))

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8), sharex=True)
    
    # Subplot 1: Ball 0 and Ball 1 (Stationary collision)
    ax1.plot(times, v_data[0], label="Ball 0 (Moving Down)", color="red", linestyle="-")
    ax1.plot(times, v_data[2], label="Ball 1 (Stationary)", color="red", linestyle="--")
    ax1.set_ylabel("Velocity ($v_z$)")
    ax1.set_title("Collision with Stationary Ball (Balls 0 & 1)")
    ax1.legend()
    ax1.grid(True)
    
    # Subplot 2: Ball 2 and Ball 3 (Opposing collision)
    ax2.plot(times, v_data[4], label="Ball 2 (Moving Down)", color="blue", linestyle="-")
    ax2.plot(times, v_data[6], label="Ball 3 (Moving Up)", color="blue", linestyle="--")
    ax2.set_xlabel("Time / Step")
    ax2.set_ylabel("Velocity ($v_z$)")
    ax2.set_title("Collision with Opposing Ball (Balls 2 & 3)")
    ax2.legend()
    ax2.grid(True)

    plt.tight_layout()
    plt.show()

if __name__ == "__main__":
    main()