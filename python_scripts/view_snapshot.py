import polars as pl
from filehandling import BatchProcess
from pathlib import Path

path_to_snapshots = "output/snapshots/"

root_path = Path(__file__).parent.parent
filepath = root_path.joinpath(path_to_snapshots,  "snapshot_00000000010.parquet")

df = pl.read_parquet(filepath)
print(df)
