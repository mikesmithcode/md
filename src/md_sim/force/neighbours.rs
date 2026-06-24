use std::cell;

use glam::DVec3;

use crate::md_sim::particle::{self, ParticleVec};
use crate::md_sim::simulation::SimulationSettings;
use crate::md_sim::force::{check_delta,Forces};


/// Optimising finding neighbours for calculation of forces
/// 
/// Divides particles in simulation up into small cells of size SimulationSettings.cutoff + SimulationSettings.skin
/// based on position coordinates.
/// Then for each particle looks at same cell and neighbouring cells and constructs
/// a verlet list of those particles that are within SimulationSettings.cutoff of the particle
/// and then the pair interaction forces are calculated using this list. In subsequent timesteps
/// the displacement of particles relative to the last time is calculated. When any particle has 
/// travelled > SimulationSettings.skin/2 the cell grid is rebuilt and the verlet lists reconstructed.
/// Particles.ptype that are not listed under SimulationSettings.active_particles are not added to the
/// verlet lists of other Particles.ptype not listed under SimulationSettings since these have dynamics
/// that are not influenced by the calculated force.
/// 
/// # Arguments
/// 
/// * `num_cells` - number cells in each dimension (calculated)
/// * `cell_size` - set by SimulationSettings.cutoff
/// * `heads` - list of the first Some(particle_id) in each cell or None if cell is empty
/// * `next` - A linked list. CellGrid.next\[i] stores the id of the next particle in the same cell as particle i
/// * stride_y, stride z - used to convert 3d to 1d particle coords.
/// * neighbour_table - relative indices of adjacent cells
/// * verlet_table - lists of particle ids within cutoff + skin for each particle
/// * skin - this is just local copy of SimulationSettings.skin
/// * last_particle_count - number of particles. If this changes we need to rebuild.
#[derive(Debug, Clone)]
pub struct CellGrid {
    pub num_cells: [usize; 3],
    pub cell_size: f64,
    pub sim_box_size: DVec3,
    pub heads: Vec<Option<usize>>, 
    pub next: Vec<Option<usize>>,  
    pub periodic: [bool;3],
    pub stride_y: usize, 
    pub stride_z: usize,
    pub neighbour_table: Vec<[usize; 26]>,
    // For the verlet lists
    pub skin: f64,
    pub verlet_table: Vec<Vec<usize>>,
    pub last_particle_count: usize,
}

    impl CellGrid {
        //----------------------------------------------------------------------
        // Public API of CellGrid
        //---------------------------------------------------------------------- 
        
        pub fn new(particle_count: usize, settings: &SimulationSettings) -> Self {
            let cell_size = settings.cutoff;
            let skin = settings.skin;
            let sim_box_size = settings.sim_box_size;
            
            let nx = ((sim_box_size.x / cell_size).floor() as usize).max(1);
            let ny = ((sim_box_size.y / cell_size).floor() as usize).max(1);
            let nz = ((sim_box_size.z / cell_size).floor() as usize).max(1);
            let total_cells = nx * ny * nz;
            let periodic = settings.periodic;


            let mut grid = Self {
                num_cells: [nx, ny, nz],
                cell_size,
                sim_box_size,
                heads: vec![None; total_cells],
                next: vec![None; particle_count],
                periodic,
                stride_y: nx,
                stride_z: nx * ny,
                neighbour_table: vec![[usize::MAX; 26]; nx * ny * nz],
                // Initialise verlet_table with one empty Vec per particle
                verlet_table: vec![Vec::with_capacity(20); particle_count],
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

        /// This is used by the Force trait to apply pairwise forces to potentially valid pairs.
        pub fn apply_pair_forces<F: Forces>(
            &self,
            f_buf: &mut [DVec3],
            t_buf: &mut [DVec3],
            particles: &ParticleVec,
            user_impl: &F,
            settings: &SimulationSettings,
        ) {
            let cutoff_sq = settings.cutoff * settings.cutoff;

            for i in 0..particles.len() {
                for &j in &self.verlet_table[i] {
                    let mut delta = particles.position[i] - particles.position[j];
                    check_delta(&mut delta,settings.sim_box_size, self.periodic);
                    
                    let dist_sq = delta.length_squared();
                    if dist_sq < cutoff_sq {
                        user_impl.update_pair_forces(i, j, f_buf, t_buf, particles, settings);
                    }
                }
            }
        }

        //-----------------------------------------------------------------------------------
        // Putting particles into a Cell based grid
        //-----------------------------------------------------------------------------------

        ///Create adjacency table. Takes into account whether periodic in a particular dimension
        // Populate the neighbour_table. Makes it easy in a 1d array to find the 
        // valid neighbours.
        pub(super) fn build_neighbour_table(&mut self){
            // All 26 possible neighbour directions in a 3D grid
            const OFFSETS: [(i32, i32, i32); 26] = [
                (1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1), // Faces
                (1, 1, 0), (1, -1, 0), (-1, 1, 0), (-1, -1, 0),                     // Edges XY
                (1, 0, 1), (1, 0, -1), (-1, 0, 1), (-1, 0, -1),                     // Edges XZ
                (0, 1, 1), (0, 1, -1), (0, -1, 1), (0, -1, -1),                     // Edges YZ
                (1, 1, 1), (1, 1, -1), (1, -1, 1), (1, -1, -1),                     // Corners
                (-1, 1, 1), (-1, 1, -1), (-1, -1, 1), (-1, -1, -1)                  // Corners
            ];

            let (nx,ny,nz) = (self.num_cells[0], self.num_cells[1], self.num_cells[2]);
            
            let mut count=0 as usize;
            for iz in 0..nz {
                for iy in 0..ny {
                    for ix in 0..nx {
                        let current_1d = self.get_1d_idx(ix, iy, iz);
                        
                        count = 0;
                        for offset in OFFSETS {
                            //in non-periodic grid some offsets are outside grid. These return None. The array is initialised with usize::MAX indicating these neighbours
                            // don't exist.
                            if let Some(n_idx) = self.get_neighbour_1d_idx(ix, iy, iz, offset) {
                                self.neighbour_table[current_1d][count]=n_idx;
                                count +=1;
                            }
                        }
                    }
                }
            }
        }

        // Put particles into cells and build linked lists associated with each cell
        pub(super) fn bin(&mut self, particles: &ParticleVec) {
            self.heads.fill(None);
            for (i, pos) in particles.position.iter().enumerate() {
                let Some((ix, iy, iz)) = self.get_3d_cell_idx(*pos) else {
                    panic!("Particle at {:?} is outside the simulation box", pos);
                };

                let cell_idx = self.get_neighbour_1d_idx(ix, iy, iz, (0,0,0)).expect("particle outside grid");
    
                self.next[i] = self.heads[cell_idx];
                self.heads[cell_idx] = Some(i);
            
            }
        }

        /// Transforms 3D grid coords to 1D memory index
        #[inline]
        pub(super) fn get_1d_idx(&self, ix: usize, iy: usize, iz: usize) -> usize {
            ix + iy * self.stride_y + iz * self.stride_z
        }

        // This returns None if particle outside simulation box and Some(ix,iy,iz) if valid. If periodic is true in a particular dimension
        // then the value will wrap.
        pub(super) fn get_neighbour_1d_idx(&self, ix: usize, iy: usize, iz: usize, offset: (i32, i32, i32)) -> Option<usize> {
            let coords = [ix as i32, iy as i32, iz as i32];
            let offsets = [offset.0, offset.1, offset.2];
            let mut new_coords = [0usize; 3];

            for i in 0..3 {
                let val = coords[i] + offsets[i];
                if self.periodic[i] {
                    new_coords[i] = val.rem_euclid(self.num_cells[i] as i32) as usize;
                } else {
                    if val < 0 || val >= self.num_cells[i] as i32 {
                        return None; // Boundary reached, no neighbour here
                    }
                    new_coords[i] = val as usize;
                }
            }
            Some(self.get_1d_idx(new_coords[0], new_coords[1], new_coords[2]))
        }

        /// Position to Coords: Transforms floating point position to 3D grid coords
        /// Includes a modulo check to ensure safety with periodic boundaries
        pub(super) fn get_3d_cell_idx(&self, pos: DVec3) -> Option<(usize, usize, usize)> {
            let dims = [self.num_cells[0], self.num_cells[1], self.num_cells[2]];
            let pos_arr = [pos.x, pos.y, pos.z];
            let mut indices = [0usize; 3];

            for i in 0..3 {
                let max_val = dims[i] as f64 * self.cell_size;
                
                if self.periodic[i] {
                    indices[i] = ((pos_arr[i] / self.cell_size).floor() as i32)
                        .rem_euclid(dims[i] as i32) as usize;
                } else {
                    // Check boundaries for non-periodic axes and panic if unallowed position found
                    if pos_arr[i] < 0.0 || pos_arr[i] >= max_val {
                        return None
                    }
                    indices[i] = (pos_arr[i] / self.cell_size) as usize;
                }
            }
            Some((indices[0], indices[1], indices[2]))
        }


        //------------------------------------------------------------------------------------
        // Everything above here is about putting particles into a cell based grid.
        // Everything below here tries to then create a verlet look up table of the 
        // particles that will have pairwise interactions
        //------------------------------------------------------------------------------------

        pub(super) fn rebuild_verlet_table(&mut self, particles: &mut ParticleVec, interaction_ptypes: &[[u8;2]]) {
            //clear previous lists
            for list in &mut self.verlet_table {
                list.clear();
            }

            let search_radius = self.cell_size + self.skin;
            let search_radius_sq = search_radius * search_radius;


            // for each cell in the grid look at other particles in the linked list
            // and check if they should be added to that particles verlet list. Then repeat
            // process for particles in the neighbouring cells. The neighbouring cell
            // might be wrapped since sim box boundaries are periodic.
            for cell_idx in 0..self.heads.len() {
                let mut i_opt = self.heads[cell_idx];
                while let Some(i) = i_opt {
                    // Look at next possible particle in same cell
                    let mut j_opt = self.heads[cell_idx]; 
                    while let Some(j) = j_opt {
                        // try_add_pair rejects particles in same molecule or wrong ptypes or the same particle or too far apart.
                        self.try_add_pair(i, j, search_radius_sq, particles, interaction_ptypes);
                        
                        j_opt = self.next[j];
                    }

                    // Look at neighbour cells
                    for neighbour_idx in self.neighbour_table[cell_idx] {
                        //Since this is a different cell start with the head particle
                        let mut j_opt = self.heads[neighbour_idx];
                        while let Some(j) = j_opt {
                            self.try_add_pair(i, j, search_radius_sq, particles, interaction_ptypes);
                            j_opt = self.next[j];
                        }
                    }
                    i_opt = self.next[i];
                    }
                }
            }

        // Check if pair should be added to particles verlet list.
        pub(super) fn try_add_pair(&mut self, i: usize, j: usize, r_sq: f64, p: &ParticleVec, interaction_ptypes: &[[u8;2]]) {
            // This prevents self-interaction (i == j) and double counting

            //if index is in same molecule ignore, which also prevents self interacting with self.
            if p.molecule_id[i] == p.molecule_id[j] {
                return;
            }

            // determine if particles are active
            let ptype_i = p.ptype[i];
            let ptype_j = p.ptype[j];

            // Check if this pair is allowed to interact based on the JSON config
            let is_pair_allowed = interaction_ptypes.iter()
                .any(|pair| pair[0] == ptype_i as u8 && pair[1] == ptype_j as u8);

            if !is_pair_allowed {
                return;
            }

            // Calculate distance
            let mut delta = p.position[i] - p.position[j];
            check_delta(&mut delta, self.sim_box_size, self.periodic);
            
            
            if delta.length_squared() < r_sq {
                // add to j to i's verlet list
                self.verlet_table[i].push(j);
            }

            
        }

        pub(super) fn resize_buffers(&mut self, new_count: usize) {
            self.next.resize(new_count, None);
            self.verlet_table.resize(new_count, Vec::with_capacity(20));
        }

    

    

}

