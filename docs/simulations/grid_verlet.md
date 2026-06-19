# Cellgrid and Verlet structure

In a simulation you need to calculate forces between pairs of particles. This is O(N^{2}) which is very expensive. You can however do it in O(N).

- Step 1 is to assign all particles to a small box which is part of a cubic grid spanning the simulation box. 

When we create the CellGrid we take coords and find index of the cubic box in each dimension e.g.

```rust
    let nx = ((box_size.x / cell_size).floor() as usize).max(1);
```

- Step 2 we find all the particles in neighbouring boxes

```rust
let mut grid = Self {
            num_cells: [nx, ny, nz],
            cell_size,
            heads: vec![None; total_cells],
            next: vec![None; particle_count],
            stride_y: nx,
            stride_z: nx * ny,
            neighbour_table: vec![Vec::new(); total_cells],
            // Initialise verlet_table with one empty Vec per particle
            verlet_table: vec![Vec::with_capacity(20); particle_count],
            skin,
            last_particle_count: 0,            
        };
```
The heads is a 1d vec with one slot per box in the cell grid. This is either empty (None) or contains the index of a particle in ParticleVec which is the "first" particle in that box. 

The next is a vector as long as the number of particles. It allows for a linked list. The second particle in a box in the cell grid has its index stored in the slot associated with the head particle. The id in the slot for the second particle is the index of the third particle. If there are no more particles in that box the value is None.

We then create a quick look up to enable us to find valid neighbouring cells in the 1d array. Effectively this is flattening the 3d grid but also taking into account whether we want periodic boundaries in each dimension or not.


