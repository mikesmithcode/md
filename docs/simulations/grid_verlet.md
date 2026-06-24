## Cellgrid and Verlet structure

In a simulation you need to calculate forces between pairs of particles. This is $\sim O(N^{2})$ which is very expensive. You can however do it in $ \sim O(N)$.

### 1. Assign all particles to a small box which is part of a cubic grid spanning the simulation box. 

When we create the CellGrid we take coords and find index of the cubic box in each dimension e.g.

### 2. Find all the particles in that box or neighbouring boxes:

We create a quick look up to enable us to find valid neighbouring cells in the 1d array `build_neighbour_table()`. Effectively this is flattening the 3d grid but also taking into account whether we want periodic boundaries in each dimension or not.

We then build a linked list of particles for each cell of the grid. 
The `heads` is a 1d vec with one slot per box in the cell grid. This is either empty (None) or contains Some(index) of a particle in ParticleVec which is the "first" particle in that box. 

The `next` is a vector as long as the number of particles. It allows for multiple linked lists. The second particle in a box in the cell grid has its index stored in the next slot associated with the head particle. The id in the slot for the second particle is the index of the third particle. If there are no more particles in that box the value is None.

The above process effectively completes assigning particles to boxes in the cell grid. 


### 3.  Build a Verlet list for each particle of all those particles that they can experience pairwise interactions. Particles can only interact if:
    - they are within a cutoff distance of each other
    - they have ptypes which are defined to interact with each other in the interaction_ptypes field of the SimulationSettings struct (constructed from the config file) 
    - they are not part of the same molecule or rigid body.

This process is handled by the `build_verlet_table()` function. The verlet table is a vector of vectors. Each particle has a vector of indices of particles that it can interact with.

This code iterates over every box in the 3d grid. For each box the fn get_1d_idx() converts the 3d coord of the box to the 1d index in the heads array. It then tests the above conditions for every particle in that box and neighbouring boxes and if it passes adds them to the verlet table for that particle.

### 4. Check for too much movement and rebuild

As particles move we monitor how far they have moved since the last time this build process was run. If any particle has moved more than half the skin distance we rebuild the verlet table. This is done by check_and rebuild_verlet_table().

Now in the Force trait we can iterate over the verlet table for each particle and calculate the pairwise forces between them. The pairs are also used in the Motion

### The API

Create a CellGrid and initialise:

```rust
let mut grid = CellGrid::new(num_particles: usize,&settings: SimulationSettings);
grid.init(&particles, interaction_ptypes: &[[u8;2]]);
```

In the loop check and if necessary rebuild the verlet table:

```rust
    grid.check_and_rebuild_neighbours(particles: &mut ParticleVec,settings: &SimulationSettings)
```

The Simulation loop will apply pairwise forces using the verlet table:

```rust
    grid.apply_pair_forces(forces: &mut [DVec3],
            torques: &mut [DVec3],
            particles: &ParticleVec,
            user_impl: &F,
            settings: &SimulationSettings,)
```
user_impl is a reference to the user defined struct which implements the Force trait. The Force trait has a function `update_pair_forces()` which is called for each pair of particles in the verlet table. The user can implement their own force calculations in this function.
