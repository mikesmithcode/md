use glam::DVec3;
use rayon::prelude::*;

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::md_sim::utils::{check_delta,InteractionContext};
use crate::md_sim::Forces;


/// A spatial hashing cell grid and Verlet list manager for accelerated pair-force calculations.
///
/// # Overview
/// 
/// `CellGrid` optimizes pairwise interaction lookups by dividing the simulation domain into a 
/// 3D grid of cells whose dimensions are governed by the interaction cutoff and skin distance. 
/// It utilizes a [**Compressed Sparse Row (CSR)** data structure](https://github.com/MikeSmithLabTeam/MD/blob/main/docs/csr.md) 
/// to store variable-length neighbor lists 
/// compactly in memory, maximizing cache locality and enabling lock-free parallelization during force loops.
///
/// ## Understanding CSR in Verlet Lists
/// 
/// In molecular simulations, different particles have varying numbers of neighbours, making standard fixed-size 
/// arrays impractical. Instead of using a vector of vectors (`Vec<Vec<usize>>`) we use a CSR. :
/// 
/// * **`verlet_particle_ids`**: A single giant array holding all neighbor IDs concatenated back-to-back.
/// * **`verlet_offsets`**: An index array of size $\text{num\_particles} + 1$ that defines slice boundaries.
/// 
/// For any particle `i`, its neighbor slice is accessed via `verlet_offsets[i]..verlet_offsets[i + 1]`. 
/// This layout provides three major advantages:
/// 1. **Cache Efficiency:** Contiguous memory allows CPU prefetchers to stream neighbor data rapidly.
/// 2. **Parallel Safety:** Because particle `i` accesses a strictly bounded, non-overlapping slice of 
///    `verlet_particle_ids` and writes to its own independent force buffer index, `.par_iter_mut()` 
///    executes safely with **zero data races or locks**.
/// 3. **Allocation Control:** Rebuilding uses a strict 3-pass pattern (Count $\rightarrow$ Prefix Sum $\rightarrow$ Fill) 
///    allowing `verlet_particle_ids` to resize with a **single exact allocation**, avoiding reallocation overhead.
///
/// # Algorithmic Workflow
///
/// 1. **Cell Binning:** Particles are assigned to grid cells based on their spatial coordinates. 
///    Periodic boundaries wrap coordinates when configured.
/// 2. **Verlet List Construction:** For each particle, the algorithm inspects its home cell and 
///    all 26 adjacent neighbor cells. A pair $(i, j)$ is added to the Verlet list if:
///    * They do not belong to the same molecule (`molecule_id[i] != molecule_id[j]`).
///    * Their particle types match an allowed entry in `interaction_ptypes`.
///    * Their separation distance is within the search radius ($\text{cutoff} + \text{skin}$).
/// 3. **Incremental Rebuilding (`check_and_rebuild_neighbours`):** To avoid rebuilding the grid 
///    every timestep, particles track a reference position (`ref_pos`). A full rebuild is only 
///    triggered if the number of particles changes or if any single particle has displaced 
///    greater than $\text{skin} / 2$ since the last rebuild.
///
/// # Important Assumptions & Behavior Notes
///
/// * **Asymmetric Interaction Matrix:** Interaction types are directional based on `interaction_ptypes`. 
///   For example, if configured as `[[0, 1]]`, particle type `1` is added to type `0`'s neighbor list, 
///   but type `0` is not necessarily added to type `1` unless `[[1, 0]]` is also explicitly specified.
/// * **Exclusion of Intramolecular Pairs:** Particles sharing the same `molecule_id` are automatically 
///   excluded from the neighbor list, as internal forces are handled separately.
/// * **Skin Buffer:** The search radius uses an expanded boundary ($\text{cutoff} + \text{skin}$) to 
///   guarantee that particles cannot drift out of range between periodic grid rebuilds.
///
/// # Fields
///
/// * `num_cells` - Number of cells along the $[x, y, z]$ dimensions.
/// * `cell_size` - Edge length of each cubic cell, set to $\text{cutoff} + \text{skin}$.
/// * `inv_cell_size` - Reciprocal of `cell_size` used for branchless multiplication during coordinate hashing.
/// * `sim_box_size` - Physical dimensions of the simulation domain.
/// * `strides` - Multipliers used to flatten 3D cell coordinates into a 1D memory index.
/// * `periodic` - Boolean flags indicating periodic boundary conditions per axis.
/// * `neighbour_table` - Precomputed table storing 1D indices of up to 26 adjacent cells for every grid cell.
/// * `cell_offsets` - Start indices in `cell_particle_ids` for each cell (CSR format, length $\text{total\_cells} + 1$).
/// * `cell_particle_ids` - Particle identifiers sorted contiguously by their current cell ownership.
/// * `skin` - Distance buffer threshold triggering a grid and Verlet list rebuild.
/// * `verlet_offsets` - Start indices in `verlet_particle_ids` for each particle (CSR format, length $\text{num\_particles} + 1$).
/// * `verlet_particle_ids` - Flattened contiguous storage of all valid neighbor pair IDs.
/// * `counts` - Temporary per-particle neighbor counts used during Verlet list allocation passes.
/// * `last_particle_count` - Tracks particle count changes to force buffer re-allocations.
#[derive(Debug, Clone)]
pub struct CellGrid {
    // Defining the grid
    pub num_cells: [usize; 3],
    pub cell_size: f64,
    pub inv_cell_size: f64,
    pub sim_box_size: DVec3,
    pub strides: [usize;3], 
    pub periodic: [bool;3],
    pub neighbour_table: Vec<Vec<usize>>,
    // For cell grid
    pub cell_offsets: Vec<usize>,       // length num_cells + 1
    pub cell_particle_ids: Vec<usize>, // length num_particles   
    // For the verlet lists
    pub skin: f64,
    pub verlet_offsets: Vec<usize>,      // length num_particles + 1
    pub verlet_particle_ids: Vec<usize>,      // length total pairs of interactions
    pub counts: Vec<usize>,              // length num_particle
    pub last_particle_count: usize,
}

    impl CellGrid {
        //----------------------------------------------------------------------
        // Public API of CellGrid
        //---------------------------------------------------------------------- 
        
        /// Creates and initializes a new cell grid spatial partitioning structure.
        ///
        /// Calculates the grid dimensions based on the simulation box size and cell size 
        /// (derived from the cutoff and skin distance), preallocating memory buffers for 
        /// cell indexing and Verlet neighbor lists.
        ///
        /// # Arguments
        ///
        /// * `particle_count` - The initial number of particles in the simulation.
        /// * `settings` - Global simulation parameters containing box dimensions, cutoff, and skin values.
        pub fn new(particle_count: usize, settings: &SimulationSettings) -> Self {
            let cell_size = settings.cutoff + settings.skin;
            let inv_cell_size = 1.0/cell_size;
            let skin = settings.skin;
            let sim_box_size = settings.sim_box_size;
            
            let nx = ((sim_box_size.x * inv_cell_size).floor() as usize).max(1);
            let ny = ((sim_box_size.y * inv_cell_size).floor() as usize).max(1);
            let nz = ((sim_box_size.z *inv_cell_size).floor() as usize).max(1);
            let total_cells = nx * ny * nz;
            let periodic = settings.periodic;
            
            let cell_offsets = vec![0; total_cells + 1];  
            let cell_particle_ids = Vec::with_capacity(particle_count);

            let counts = vec![0; particle_count];
            let verlet_offsets = vec![0; particle_count + 1];  
            let verlet_particle_ids = Vec::with_capacity(12*particle_count);

            let mut grid = Self {
                num_cells: [nx, ny, nz],
                cell_size,
                inv_cell_size,
                sim_box_size,
                strides: [1,nx,nx*ny],
                periodic,
                neighbour_table: vec![Vec::new(); total_cells],
                cell_offsets,
                cell_particle_ids,
                verlet_offsets,      
                verlet_particle_ids,      
                counts,             
                skin,
                last_particle_count: particle_count,            
            };

            grid.build_neighbour_table();

            grid
        }

        /// Initializes the cell grid and builds initial Verlet lists upon simulation startup.
        ///
        /// # Arguments
        ///
        /// * `particles` - Mutable reference to the particle data structures.
        /// * `settings` - Global simulation parameters.
        pub fn init(&mut self, particles: &mut ParticleVec, settings: &SimulationSettings){
            self.bin(particles);
            particles.ref_pos.copy_from_slice(&particles.position);
            self.rebuild_verlet_table(particles, &settings.interaction_ptypes);
        }

        /// Checks particle displacement and updates neighbour lists if necessary.
        ///
        /// This evaluates whether any particle has drifted more than half the skin distance ($\text{skin} / 2$) 
        /// from its recorded reference position, or if the total particle count has changed. If either condition 
        /// is met, the cell grid and Verlet tables are fully rebuilt.
        ///
        /// # Arguments
        ///
        /// * `particles` - Mutable reference to the particle buffers containing current positions, reference positions, and counts.
        /// * `settings` - Global simulation parameters containing skin thickness, cutoffs, and allowed interaction types.
        ///
        /// # Notes
        ///
        /// * **Skin Distance Threshold:** The squared displacement threshold is evaluated as $(\text{skin} \times 0.5)^2$ 
        ///   to ensure particles cannot cross into new interaction zones between periodic rebuilds.
        /// * **State Sync:** Upon rebuilding, reference positions (`ref_pos`) are re-synced with current positions, 
        ///   and particle count trackers are updated.
        pub fn check_and_rebuild_neighbours(&mut self,particles: &mut ParticleVec,settings: &SimulationSettings) {
            let threshold_sq = (settings.skin * 0.5).powi(2);

            // Check if the number of particles has changed
            let count_changed = particles.len() != self.last_particle_count;
            
            // Check if any particle has moved too far
            let moved_too_far = particles.position.iter()
                .zip(particles.ref_pos.iter())
                .any(|(p, r)| {
                    let mut delta = *p - *r;
                    check_delta(&mut delta, self.sim_box_size, self.periodic);
                    delta.length_squared() > threshold_sq
                });

            if count_changed || moved_too_far {
                // Ensure our internal buffers (next, verlet_table) match the new size
                if count_changed {
                    self.resize_buffers(particles.len());
                }
                
                self.bin(particles);
                self.rebuild_verlet_table(particles, &settings.interaction_ptypes);
                // Sync the reference positions
                particles.ref_pos.copy_from_slice(&particles.position);
                
                // Update the count tracker
                self.last_particle_count = particles.len();
            }
        }

        /// Computes all pairwise interactions in parallel using the Verlet neighbor lists.
        ///
        /// Iterates over each particle's neighbor slice via CSR offset lookups and invokes 
        /// the user-defined force evaluation model.
        ///
        /// # Arguments
        ///
        /// * `f_buf` - Mutable slice for accumulating force vectors per particle.
        /// * `t_buf` - Mutable slice for accumulating torque vectors per particle.
        /// * `particles` - Read-only reference to particle data.
        /// * `user_impl` - The user-provided type implementing the `Forces` trait.
        /// * `settings` - Global simulation parameters.
       pub fn apply_pair_forces<F: Forces + Sync>(
            &self,
            f_buf: &mut [DVec3],
            t_buf: &mut [DVec3],
            particles: &ParticleVec,
            user_impl: &F,
            settings: &SimulationSettings,
        ) {
            // We iterate over particle indices (i)
            f_buf.par_iter_mut()
                .zip(t_buf.par_iter_mut())
                .enumerate()
                .for_each(|(i, (f_out, t_out))| {
                    let mut local_force = DVec3::ZERO;
                    let mut local_torque = DVec3::ZERO;

                    // CSR Access: Look up the range for particle i
                    let start = self.verlet_offsets[i];
                    let end = self.verlet_offsets[i + 1];
                    
                    // Iterate over the slice of neighbours directly
                    for &j in &self.verlet_particle_ids[start..end] {
                        let (f, t) = user_impl.update_pair_forces(
                            i, j, DVec3::ZERO, DVec3::ZERO, particles, settings
                        );
                        local_force += f;
                        local_torque += t;
                    }

                    *f_out += local_force;
                    *t_out += local_torque;
                });
        }

        //-----------------------------------------------------------------------------------
        // Putting particles into a Cell based grid
        //-----------------------------------------------------------------------------------

        // Populate the neighbour_table. Makes it easy in a 1d array to find the 
        // valid neighbours, handling boundary wrapping rules automatically.
        pub(super) fn build_neighbour_table(&mut self) {
            const OFFSETS: [[i32;3]; 26] = [
                [1, 0, 0], [-1,0, 0], [0, 1, 0], [0,-1, 0], [0, 0, 1], [0, 0, -1],
                [1, 1, 0], [1,-1, 0], [-1, 1,0], [-1,-1,0],                     
                [1, 0, 1], [1, 0,-1], [-1, 0,1], [-1,0,-1],                     
                [0, 1, 1], [0, 1,-1], [0, -1,1], [0,-1,-1],                     
                [1, 1, 1], [1, 1,-1], [1, -1,1], [1,-1,-1],                     
                [-1, 1,1], [-1,1,-1], [-1,-1,1], [-1,-1,-1]                  
            ];

            let (nx, ny, nz) = (self.num_cells[0], self.num_cells[1], self.num_cells[2]);
            self.neighbour_table = vec![Vec::new(); nx * ny * nz];

            for iz in 0..nz {
                for iy in 0..ny {
                    for ix in 0..nx {
                        let current_1d = self.get_1d_idx(ix, iy, iz);
                        
                        for offset in OFFSETS {
                            let n_idx = self.get_neighbour_1d_idx(ix, iy, iz, offset);
                            if n_idx != usize::MAX {
                                self.neighbour_table[current_1d].push(n_idx);
                            }
                        }
                    }
                }
            }
        }

        // Put particles into cells and build CSR array
        pub(super) fn bin(&mut self, particles: &ParticleVec) {
            // Reset cell counts (reuse a temporary buffer or use self.cell_offsets)
            let mut cell_counts = vec![0; self.num_cells[0] * self.num_cells[1] * self.num_cells[2]];
            
            // Count particles in each cell
            for pos in particles.position.iter() {
                let cell_idx = self.get_cell_idx_from_pos(pos); 
                cell_counts[cell_idx] += 1;
            }

            // Prefix sum to get the starting offset for each cell
            self.cell_offsets[0] = 0;
            for i in 0..cell_counts.len() {
                self.cell_offsets[i + 1] = self.cell_offsets[i] + cell_counts[i];
            }

            // Populate cell_particle_indices
            // Create a local tracker to fill slots within the pre-calculated ranges
            let mut current_pos = self.cell_offsets.clone(); 
            
            // Ensure the array is sized correctly
            self.cell_particle_ids.resize(particles.position.len(), 0);

            for (i, pos) in particles.position.iter().enumerate() {
                let cell_idx = self.get_cell_idx_from_pos(pos);
                
                // Write particle index into the reserved block for this cell
                let target_idx = current_pos[cell_idx];
                self.cell_particle_ids[target_idx] = i;
                
                // Increment the tracker for the next particle in this same cell
                current_pos[cell_idx] += 1;
            }
        }

        // Computes the 1D linear cell array index corresponding to a given 3D position vector.
        #[inline(always)]
        pub(super) fn get_cell_idx_from_pos(&self, pos: &DVec3) -> usize {
            let x = (pos.x * self.inv_cell_size) as usize;
            let y = (pos.y * self.inv_cell_size) as usize;
            let z = (pos.z * self.inv_cell_size) as usize;
            
            let ix = x.min(self.num_cells[0] - 1);
            let iy = y.min(self.num_cells[1] - 1);
            let iz = z.min(self.num_cells[2] - 1);

            ix + iy * self.strides[1] + iz * self.strides[2]
        }

        /// Transforms 3D grid coords to 1D memory index
        #[inline(always)]
        pub(super) fn get_1d_idx(&self, ix: usize, iy: usize, iz: usize) -> usize {
            ix + iy * self.strides[1] + iz * self.strides[2]
        }

        // Computes the 1D linear index for a neighboring cell given base coordinates and an offset.
        // Returns usize::MAX if the neighbor falls outside the simulation box in a non-periodic dimension, 
        // or wraps correctly using Euclidean modulo if periodic is enabled.
        #[inline(always)]
        pub(super) fn get_neighbour_1d_idx(&self, ix: usize, iy: usize, iz: usize, offsets: [i32;3]) -> usize {
            let mut coords = [ix as i32, iy as i32, iz as i32];

            for i in 0..3 {
                let val = coords[i] + offsets[i];
                
                if self.periodic[i] {
                    coords[i]=val.rem_euclid(self.num_cells[i] as i32);
                } else {
                    // Clamping is branchless on most modern CPUs (min/max instructions)
                    if val < 0 || val >= self.num_cells[i] as i32 {
                        return usize::MAX; 
                    }
                    coords[i] = val;
                };
            }
            self.get_1d_idx(coords[0] as usize, coords[1] as usize, coords[2] as usize)
        }

        // Called if the number of particles in the simulation has changed.
        fn resize_buffers(&mut self, particle_count: usize){
            self.counts.resize(particle_count, 0);
            self.verlet_offsets.resize(particle_count + 1, 0);
            self.verlet_particle_ids.clear();
        }

        //------------------------------------------------------------------------------------
        // Everything above here is about putting particles into a cell based grid.
        // Everything below here tries to then create a verlet look up table of the 
        // particles that will have pairwise interactions
        //------------------------------------------------------------------------------------
        
        // Builds the Verlet lookup table using a 3-pass counter, prefix-sum, and population scheme.
        pub(super) fn rebuild_verlet_table(&mut self, particles: &ParticleVec, interaction_ptypes: &[[u8; 2]]) {
            let int_context = InteractionContext {
                sim_box_size: self.sim_box_size,
                periodic: self.periodic,
                search_radius_sq: (self.cell_size + self.skin).powi(2),
                interaction_ptypes,
            };

            let offsets = &self.cell_offsets;
            let indices = &self.cell_particle_ids;
            let neighbours = &self.neighbour_table;

            // Reset counts
            self.counts.fill(0);

            // --- PASS 1: Count ---
            for cell_idx in 0..offsets.len() - 1 {
                let range = offsets[cell_idx]..offsets[cell_idx + 1];
                for &i in &indices[range.clone()] {
                    // Same cell
                    for &j in &indices[range.clone()] {
                        if Self::add_to_verlet(i, j, particles, &int_context) {
                            self.counts[i] += 1;
                        }
                    }
                    // Neighbour cells
                    for &n_idx in &neighbours[cell_idx] {
                        let n_range = offsets[n_idx]..offsets[n_idx + 1];
                        for &j in &indices[n_range] {
                            if Self::add_to_verlet(i, j, particles, &int_context) {
                                self.counts[i] += 1;
                            }
                        }
                    }
                }
            }

            // --- PASS 2: Prefix Sum ---
            self.verlet_offsets[0] = 0;
            for i in 0..particles.position.len() {
                self.verlet_offsets[i + 1] = self.verlet_offsets[i] + self.counts[i];
            }
            
            // Resize indices buffer if total count increased
            let total_pairs = self.verlet_offsets[particles.position.len()];
            self.verlet_particle_ids.resize(total_pairs, 0);

            // --- PASS 3: Fill ---
            let mut current_pos = self.verlet_offsets.clone();
            for cell_idx in 0..offsets.len() - 1 {
                let range = offsets[cell_idx]..offsets[cell_idx + 1];
                for &i in &indices[range.clone()] {
                    // Same cell
                    for &j in &indices[range.clone()] {
                        if Self::add_to_verlet(i, j, particles, &int_context) {
                            self.verlet_particle_ids[current_pos[i]] = j;
                            current_pos[i] += 1;
                        }
                    }
                    // Neighbors
                    for &n_idx in &neighbours[cell_idx] {
                        let n_range = offsets[n_idx]..offsets[n_idx + 1];
                        for &j in &indices[n_range] {
                            if Self::add_to_verlet(i, j, particles, &int_context) {
                                self.verlet_particle_ids[current_pos[i]] = j;
                                current_pos[i] += 1;
                            }
                        }
                    }
                }
            }
        }

    // Logic here:
    // 1. Check if part of same molecule and ignore if so (this excludes i==j)
    // 2. Check if the interaction_ptype includes this pair
    // 3. Check if the separation means they should be included.
    #[inline(always)]
    pub (super) fn add_to_verlet(i: usize, j: usize, p: &ParticleVec, ctx: &InteractionContext) -> bool {
            if p.molecule_id[i] == p.molecule_id[j] { return false; }

            let ptype_i = p.ptype[i];
            let ptype_j = p.ptype[j];

            let is_pair_allowed = ctx.interaction_ptypes.iter()
                .any(|pair| pair[0] == ptype_i as u8 && pair[1] == ptype_j as u8);

            if !is_pair_allowed { return false; }

            let mut delta = p.position[i] - p.position[j];
            check_delta(&mut delta, ctx.sim_box_size, ctx.periodic);
            
            delta.length_squared() < ctx.search_radius_sq
        }  

}

