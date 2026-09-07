"""Setup script for coeff"""
import polars as pl
import matplotlib
import numpy as np
matplotlib.use('qtAgg')


from utils.file_io import get_config
from utils.graphics import display
from utils.particles_objects import generate_molecules, create_rectangle


config, particles_filepath, objects_filepath = get_config()
print(objects_filepath)
box = config["sim_box_size"]
print(box)


#sinusoidally vibrating rectangle surface
w=box[0]
d=box[1]
h = box[2]

z=0.005
rect = [(0.0,0.0,z),(0.0,d,z),(w,d,z),(w,0.0,z)]

# Create the rectangle DataFrame
rect_df = create_rectangle(
    vertices=rect, 
    id_val=0, 
    colour=(0, 255, 0, 255) # Optional styling color
)
rect_df.write_parquet(objects_filepath)


r_ball = 0.0025
dr_ball = 0.25

pos_template = [(r_ball + r_ball*2*i, d/2, h-r_ball) for i in range(11)]
positions = []
for j in range(8):
    if j%2==0:
        pos1 = [(pos[0] + r_ball, pos[1], h-(2*j + 1)*r_ball) for pos in pos_template]
    if j%2==1:
        pos1 = [(pos[0], pos[1], h-(2*j + 1)*r_ball) for pos in pos_template]
    positions.extend(pos1)

rads = [r_ball - dr_ball * r_ball * np.random.uniform(1.0, 0.0) for i in range(len(positions))]


d_r=0.75





molecules = list(generate_molecules(positions, rad=rads,d_r=d_r, ptype=0))


df = pl.concat(molecules)
df.write_parquet(particles_filepath)






print(f"Successfully initialised {len(df)} particles for a {box[0]}x{box[2]} box.")
print(df['ptype','charge'].head())
display(df, box, objects_df=rect_df)
