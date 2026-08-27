"""This is a utility script for looking at output"""

import polars as pl
from pathlib import Path

script = "testnormalcollisions"

path_to_snapshots = Path("output/" + script + "/" + script + "/particles")

root = Path(__file__).parent.parent
print(root)
filepath = root.joinpath(path_to_snapshots, "particles_0000000000.parquet")
print("filepath", filepath)
df = pl.read_parquet(filepath)
print(df[['id','vz','ptype','x','z']])
