use glam::DVec3;
use rayon::prelude::*;

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::simulation::SimulationSettings;
use crate::md_sim::utils::{check_delta,InteractionContext};
use crate::md_sim::Forces;


/// Optimising finding neighbours for calculation of forces
/// 
/// Divides particles in simulation up into small cells of size SimulationSettings.cutoff + SimulationSettings.skin
/// based on position coordinates.
/// Then for each particle looks at same cell and neighbouring cells and constructs
/// a verlet list of those particles that are within SimulationSettings.cutoff of the particle
/// and then the pair interaction forces are calculated using this list. In subsequent timesteps
/// the displacement of particles relative to the last time is calculated. When any particle has 
/// travelled > SimulationSettings.skin/2 the cell grid is rebuilt and the verlet lists reconstructed.
/// Particles.ptype that are not listed under SimulationSettings.interaction_ptypes are not added to the
/// verlet lists. The structure is asymmetric. ie if interaction_ptypes = [[0,0],[0,1]] ptype 1 will be added to ptype 0's verlet list but not vice a versa. This would need a [1,0].Any particles that are part of the same molecule are also not included since these will be dealt with by the internal forces section.
/// 
/// # Arguments
/// 
/// * `num_cells` - number cells in each dimension (calculated)
/// * `cell_size` - set by SimulationSettings.cutoff
/// * `inv_cell_size` - calculated to enable multiply rather than divide which is faster.
/// * `sim_box_size`  - dimensions of the simulation box in each dimension derived from SimulationSettings
/// * strides - used to convert 3d to 1d particle coords.
/// * neighbour_table - relative indices of adjacent cells
/// * cell_offsets - The indices in cell_particle_ids at which a particular cell starts
/// * cell_particle_ids - list of particle ids arranged by which cell they are in.
/// * skin - this is just local copy of SimulationSettings.skin. When any particle has travelled skin/2 this triggers a rebuild.
/// * verlet_offsets - lists of particle ids within cutoff + skin for each particle
/// * verlet_particle_ids - list of ids which are neighbours to a particle. So the array starts with particle_id=0's neighbours then particle_id=1's neighbours. The start and finish of each particles neighbour ids is stored sequentially in verlet_offsets.
/// * last_particle_count - number of particles. If this changes we need to rebuild.
#[derive(Debug, Clone)]
pub struct CellGrid {
    // Defining the grid
    pub num_cells: [usize; 3],
    pub cell_size: f64,
    pub inv_cell_size: f64,
    pub sim_box_size: DVec3,
    pub strides: [usize;3], 
    pub periodic: [bool;3],
    pub neighbour_table: Vec<[usize; 26]>,
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
                neighbour_table: vec![[usize::MAX; 26]; nx * ny * nz],
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

        /// Called when Simulation is initialised
        pub fn init(&mut self, particles: &mut ParticleVec, settings: &SimulationSettings){
            self.bin(particles);
            particles.ref_pos.copy_from_slice(&particles.position);
            self.rebuild_verlet_table(particles, &settings.interaction_ptypes);
        }

        /// check and rebuild neighbours
        /// 
        /// This checks whether any particle has moved more than the skin/2. If so then the cell grid and verlet lists
        /// are rebuilt. If the number of particles has changed we also rebuild everything.
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

       /// This is where all pairwise interactions are applied. This is called via the simulation loop. For each 
       /// particle i we look up its neighbours in the verlet lists and apply the forces specified in the user_impl's 
       /// trait implementations (This is the struct definitions implemented at the top of a simulation script).
       /// This is done in parallel to speed things up as this is the most memory hungry bit of the simulation.
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

        ///Create adjacency table. Takes into account whether periodic in a particular dimension
        // Populate the neighbour_table. Makes it easy in a 1d array to find the 
        // valid neighbours.
        pub(super) fn build_neighbour_table(&mut self){
            // All 26 possible neighbour directions in a 3D grid
            const OFFSETS: [[i32;3]; 26] = [
                [1, 0, 0], [-1,0, 0], [0, 1, 0], [0,-1, 0], [0, 0, 1], [0, 0, -1], // Faces
                [1, 1, 0], [1,-1, 0], [-1, 1,0], [-1,-1,0],                     // Edges XY
                [1, 0, 1], [1, 0,-1], [-1, 0,1], [-1,0,-1],                     // Edges XZ
                [0, 1, 1], [0, 1,-1], [0, -1,1], [0,-1,-1],                     // Edges YZ
                [1, 1, 1], [1, 1,-1], [1, -1,1], [1,-1,-1],                     // Corners
                [-1, 1,1], [-1,1,-1], [-1,-1,1], [-1,-1,-1]                  // Corners
            ];

            let (nx,ny,nz) = (self.num_cells[0], self.num_cells[1], self.num_cells[2]);
            
            let mut count: usize;
            for iz in 0..nz {
                for iy in 0..ny {
                    for ix in 0..nx {
                        let current_1d = self.get_1d_idx(ix, iy, iz);
                        
                        count = 0;
                        for offset in OFFSETS {
                            //in non-periodic grid some offsets are outside grid. These return None. The array is initialised with usize::MAX indicating these neighbours
                            // don't exist.
                            let n_idx = self.get_neighbour_1d_idx(ix, iy, iz, offset);
                            if n_idx != usize::MAX {
                                self.neighbour_table[current_1d][count]=n_idx;
                                count +=1;
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

        // This returns None if particle outside simulation box and Some(ix,iy,iz) if valid. If periodic is true in a particular dimension
        // then the value will wrap.
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
        
        // builds the verlet table list.
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

