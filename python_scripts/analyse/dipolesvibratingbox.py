from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np
import polars as pl
import sys



def load_simulation_data() -> pl.DataFrame:
    # Extract $INPUT_NAME passed from the bash wrapper (e.g. vibratingdipoles_dr0.0)
    input_name = sys.argv[1] if len(sys.argv) > 1 else Path(__file__).stem

    # script_name becomes base name like 'vibratingdipoles' (derived from the file name, e.g. 'analyse_vibratingdipoles.py')
    script_name = Path(__file__).stem.replace("analyse", "").strip("_")

    # script_name_with_ending uses the full $INPUT_NAME passed via command line
    script_name_with_ending = input_name

    particles_dir = Path(__file__).parents[2].joinpath(
        "output", script_name, script_name_with_ending, "particles")

    print(script_name)
    print(particles_dir)

    if not particles_dir.exists():
        raise FileNotFoundError(f"Directory does not exist: {particles_dir}")

    files = sorted(particles_dir.glob("particles_*.parquet"))
    if not files:
        raise FileNotFoundError(f"No parquet files found in: {particles_dir}")

    dfs_all = [pl.read_parquet(file) for file in files]
    return pl.concat(dfs_all)


def plot_quaternion(df_all: pl.DataFrame, wrapped=True, num=2) -> None:
  """Method 2: Rotation angle from quaternions (ptype = 0)."""
  df_big = df_all.filter(pl.col("ptype") == 0).sort("t")

  plt.figure(num, figsize=(10, 6))
  for key, group in df_big.group_by("id", maintain_order=True):
    t = group["t"].to_numpy()
    qw = group["qw"].to_numpy()
    qy = group["qy"].to_numpy()

    raw_angle = 2 * np.arctan2(qy, qw)

    if wrapped:
      # Cleanly map [-2pi, 2pi] into [-pi, pi] matching the charge method
      angle = (raw_angle + np.pi) % (2 * np.pi) - np.pi
      
      # Insert NaNs where the wrapped angle jumps across the pi/-pi boundary
      jumps = np.abs(np.diff(angle)) > np.pi
      if np.any(jumps):
        t = np.insert(t, np.where(jumps)[0] + 1, np.nan)
        angle = np.insert(angle, np.where(jumps)[0] + 1, np.nan)
    else:
      angle = np.unwrap(raw_angle)

    plt.plot(t, angle, alpha=0.7, lw=1.2)

  plt.xlabel("Time (t)", fontsize=12)
  ylabel_text = (
      "Rotation Angle about Y (radians - wrapped)"
      if wrapped
      else "Absolute Rotation Angle about Y (radians - unwrapped)"
  )
  plt.ylabel(ylabel_text, fontsize=12)
  title_text = (
      "Quaternion Wrapped Rotation Traces (ptype = 0)"
      if wrapped
      else "Quaternion Total Absolute Rotation Traces (ptype = 0)"
  )
  plt.title(title_text, fontsize=14)
  plt.grid(True, linestyle="--", alpha=0.5)
  plt.tight_layout()

def plot_charge_position(
    df_all: pl.DataFrame, wrapped=True, zero_start=True, num=1
) -> None:
    """Method 3: Rotation angle tracked via the internal charge position (ptype = 1)

    relative to the big sphere (ptype = 0) using molecule_id.
    """
    df_big = df_all.filter(pl.col("ptype") == 0).select(
        ["t", "molecule_id", "id", "x", "z"]
    )
    df_charge = df_all.filter(pl.col("ptype") == 1).select(
        ["t", "molecule_id", "x", "z"]
    )

    df_joined = (
        df_charge.join(df_big, on=["t", "molecule_id"], suffix="_big")
        .with_columns(
            dx=pl.col("x") - pl.col("x_big"),
            dz=pl.col("z") - pl.col("z_big"),
        )
        .sort("t")
    )

    plt.figure(num, figsize=(10, 6))
    for key, group in df_joined.group_by("molecule_id", maintain_order=True):
        t = group["t"].to_numpy()
        #inverted the sign here to make it the same direction as the quaternion function
        raw_angle = -np.arctan2(group["dz"].to_numpy(), group["dx"].to_numpy())

        if wrapped:
            angle = raw_angle
            if zero_start and len(angle) > 0:
                angle = angle - angle[0]
                angle = (angle + np.pi) % (2 * np.pi) - np.pi

            jumps = np.abs(np.diff(angle)) > np.pi
            if np.any(jumps):
                t = np.insert(t, np.where(jumps)[0] + 1, np.nan)
                angle = np.insert(angle, np.where(jumps)[0] + 1, np.nan)
        else:
            angle = np.unwrap(raw_angle)
            if zero_start and len(angle) > 0:
                angle = angle - angle[0]

        plt.plot(t, angle, alpha=0.7, lw=1.2)

    plt.xlabel("Time (t)", fontsize=12)
    ylabel_text = (
        "Rotation Angle (radians - wrapped)"
        if wrapped
        else "Absolute Rotation Angle (radians - unwrapped)"
    )
    plt.ylabel(ylabel_text, fontsize=12)
    title_text = (
        "Particle Rotation via Charge Position (Wrapped)"
        if wrapped
        else "Particle Rotation via Charge Position (Unwrapped)"
    )
    plt.title(title_text, fontsize=14)
    plt.grid(True, linestyle="--", alpha=0.5)
    plt.tight_layout()


if __name__ == "__main__":
    input_name = sys.argv[1] if len(sys.argv) > 1 else Path(__file__).stem
    script_name = Path(__file__).stem.replace("analyse", "").strip("_")
    script_name_with_ending = input_name

    # Set up the graphs directory inside output/script_name/graphs
    graphs_dir = Path(__file__).parents[2].joinpath("output", script_name, "graphs")
    graphs_dir.mkdir(parents=True, exist_ok=True)

    df_particles = load_simulation_data()

    # Generate and save quaternion plot (Figure 2)
    plot_quaternion(df_particles, num=2)
    quat_path = graphs_dir / f"{script_name_with_ending}_quaternion.png"
    plt.figure(2)
    plt.savefig(quat_path, dpi=300)

    # Generate and save charge position plot (Figure 3)
    #plot_charge_position(df_particles, num=3)
    #charge_path = graphs_dir / f"{script_name_with_ending}_charge.png"
    #plt.figure(3)
    #plt.savefig(charge_path, dpi=300)

    plt.show()