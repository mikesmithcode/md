"""Utility"""

import polars as pl
import matplotlib.pyplot as plt
import numpy as np
from filehandling import BatchProcess
from scipy.signal import find_peaks

path = "output/coeff_restitution"
path_to_snapshots = path + "/snapshots"

pk_threshold=0.005

t=[]
z=[]
for file in BatchProcess(path_to_snapshots + "/*.parquet"):
    df = pl.read_parquet(file)
    # Filter for ptype 1
    result = df.filter(pl.col("ptype") == 1).select(["t", "z"])

    # Extract the values (assuming exactly one row exists)
    t.append(result["t"].item())
    z.append(result["z"].item())


h1 = z[0]

t=np.array(t)
z=np.array(z)

t_peaks, properties =find_peaks(z, height=pk_threshold)
z_peaks = properties["peak_heights"]

t_peaks = np.append(0,t_peaks)
z_peaks = np.append(z[0], z_peaks)

print(z_peaks[1:]/z_peaks[:-1])

fig,axes =plt.subplots(ncols=1, nrows=2)
axes[0].plot(t,z,'r.')

axes[0].plot(t[t_peaks],z_peaks,'bo')
axes[0].set_xlabel("time (s)")
axes[0].set_ylabel("height (m)")
axes[1].plot(z_peaks[1:]**0.5, z_peaks[:-1]**0.5)
axes[1].set_xlabel("h2^0.5 (m^0.5)")
axes[1].set_ylabel("h1^0.5 (m^0.5)")
plt.show()



