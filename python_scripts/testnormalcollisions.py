"""Setup script for coeff"""
import polars as pl
import matplotlib
matplotlib.use('qtAgg')


from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import generate_molecules, create_rectangle


config, particles_filepath, objects_filepath = get_config()
print(objects_filepath)
box = config["sim_box_size"]
print(box)

#Two moving particles and one static
positions = [(0.02,0.025,0.03), (0.04,0.025,0.03), (0.04, 0.025, 0.005)]
velocities = [(0.0,0.0,-10.0),(0.0,0.0,-10.0), (0.0,0.0,0.0)]
radii = [0.0005, 0.0005, 0.005]
ptypes = [0, 0, 2]

d_r = [0.5,0.5,0.5]

molecules = list(generate_molecules(positions, v=velocities, d_r=d_r, rad=radii, ptype=ptypes))


df = pl.concat(molecules)
df.write_parquet(particles_filepath)

#Rectangle fills half the box.
w=box[0]/2.0
h=box[1]
z=0.01
rect = [(0.0,0.0,z),(0.0,h,z),(w,h,z),(w,0.0,z)]
# Create the rectangle DataFrame
rect_df = create_rectangle(
    vertices=rect, 
    id_val=0, 
    colour=(0, 255, 0, 255) # Optional styling color
)
rect_df.write_parquet(objects_filepath)



print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype','charge'].head())
display(df, box, objects_df=rect_df)
