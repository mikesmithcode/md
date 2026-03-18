use glam::DVec3;
use crate::md_sim::particle::ParticleVec;
use crate::md_sim::simulation::SimulationSettings;
use crate::md_sim::force::Forces;

#[derive(Debug, Clone)]
pub struct CellGrid {
    pub num_cells: [usize; 3],
    pub cell_size: f64,
    pub heads: Vec<Option<usize>>, // Points to the first particle index in each cell
    pub next: Vec<Option<usize>>,  // Points to the next particle index in the chain
    // Pre-calculate strides for the flattening formula
    pub stride_y: usize, 
    pub stride_z: usize,
    pub neighbour_table: Vec<Vec<usize>>,
}


impl CellGrid{

    /// Setup the CellGrid
    /// 
    /// The CellGrid provides a data structure for efficiently reducing the number of
    /// particles we need to check for things like collisions.
    pub fn new(box_size: DVec3, cell_size: f64, particle_count: usize) -> Self {
        // Calculate how many cells fit in each dimension. Ensure there is at least 1.
        let nx = ((box_size.x / cell_size).floor() as usize).max(1);
        let ny = ((box_size.y / cell_size).floor() as usize).max(1);
        let nz = ((box_size.z / cell_size).floor() as usize).max(1);
        let total_cells = nx * ny * nz;

        const OFFSETS:   [(i32, i32, i32); 13] = [
            (1, 0, 0),  (1, 1, 0),  (0, 1, 0), (-1, 1, 0),
            (1, 0, 1),  (1, 1, 1),  (0, 1, 1), (-1, 1, 1),
            (0, 0, 1),  (1, -1, 1), (0, -1, 1), (-1, -1, 1),
            (-1, 0, 1)
        ];

        let mut grid = Self {
            num_cells: [nx, ny, nz],
            cell_size,
            heads: vec![None; total_cells],
            next: vec![None; particle_count],
            neighbour_table: vec![Vec::new(); total_cells],
            stride_y: nx,
            stride_z: nx * ny,
        };

        // Populate the grid
        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let current_1d = grid.get_1d_idx(ix, iy, iz);
                    let mut unique_neighbours = Vec::new();

                    for offset in OFFSETS {
                        // Calculate wrapped coords (e.g., ix + 1, iy - 1, etc.)
                        let n_idx = grid.get_wrapped_1d_idx(ix, iy, iz, offset);
                        // Add if unique
                        if n_idx != current_1d && !unique_neighbours.contains(&n_idx) {
                            unique_neighbours.push(n_idx);
                        }
                    }
                    grid.neighbour_table[current_1d] = unique_neighbours;
                }
            }
        }
        grid
    
    }

    /// Bin particles into a particular bin based on their position
    /// 
    /// First calculate the 3d coord of the bin from particle position.
    /// Translate that 3d coord into a 1d index into the heads array.
    /// The heads array stores the first particle in each cell.
    /// So heads[cell_idx] returns Some(i) where i is the index in the 
    /// ParticleVec corresponding to a specific particle.
    /// 
    /// Each cell can contain more than one particle.
    /// In this case you have a single head particle
    /// and then this is part of a linked list.
    /// The next array when indexed by a particle id
    /// tells you which particle id is next in your linked
    /// list. At the end of the linked list calling
    /// next will return None.
    pub fn bin(&mut self, particles: &ParticleVec){
        self.heads.fill(None);
        
        for (i, pos) in particles.position.iter().enumerate(){
            let (ix,iy,iz) = self.get_3d_cell_idx(*pos);
            let cell_idx = self.get_1d_idx(ix,iy,iz);
            self.next[i] = self.heads[cell_idx];
            self.heads[cell_idx] = Some(i);
        }
    }

    /// Walk the grid and call the user's physics implementation for every valid pair.
    /// This uses the pre-calculated 13-neighbour table to avoid double-counting.
    pub fn search_and_apply_pair_forces<F: Forces>(
        &self,
        f_buf: &mut [DVec3],
        particles: &ParticleVec,
        user_impl: &F,
        settings: &SimulationSettings,
    ) {
        // Iterate through every cell in the grid (1D index is sufficient here)
        for cell_idx in 0..self.heads.len() {
            
            // i_opt is the "Optional" head of the current cell's linked list
            let mut i_opt = self.heads[cell_idx];

            // Walk the chain for particle 'i'
            while let Some(i) = i_opt {
                
                // --- Search A: Same-Cell Pairs ---
                // We start 'j' at the particle AFTER 'i' in the same list.
                // This ensures we check every pair once and never i against i.
                let mut j_opt = self.next[i];
                while let Some(j) = j_opt {
                    user_impl.update_pair_forces(i, j, f_buf, particles, settings);
                    j_opt = self.next[j];
                }

                // --- Search B: Neighbour-Cell Pairs ---
                // We look at all particles in the 13 unique neighbour cells.
                // The 'unique' check in the constructor handles thin/2D boxes.
                for &neighbour_idx in &self.neighbour_table[cell_idx] {
                    let mut nj_opt = self.heads[neighbour_idx];
                    
                    while let Some(j) = nj_opt {
                        user_impl.update_pair_forces(i, j, f_buf, particles, settings);
                        nj_opt = self.next[j];
                    }
                }

                // Move i_opt to the next particle in the current cell's chain
                i_opt = self.next[i];
            }
        }
    }


/// Transforms 3D grid coords to 1D memory index
    #[inline]
    fn get_1d_idx(&self, ix: usize, iy: usize, iz: usize) -> usize {
        ix + iy * self.stride_y + iz * self.stride_z
    }

    /// Position to Coords: Transforms floating point position to 3D grid coords
    /// Includes a modulo check to ensure safety with periodic boundaries
    fn get_3d_cell_idx(&self, pos: DVec3) -> (usize, usize, usize) {
        let x_cells = self.num_cells[0];
        let y_cells = self.num_cells[1];
        let z_cells = self.num_cells[2];

        // Ensure we handle potential negative positions or edge cases with modulo
        let idx = ((pos.x / self.cell_size).floor() as i32).rem_euclid(x_cells as i32) as usize;
        let idy = ((pos.y / self.cell_size).floor() as i32).rem_euclid(y_cells as i32) as usize;
        let idz = ((pos.z / self.cell_size).floor() as i32).rem_euclid(z_cells as i32) as usize;

        (idx, idy, idz)
    }

    /// The Neighbour Indexer: Handles offsets (dx, dy, dz) across periodic boundaries
    fn get_wrapped_1d_idx(&self, ix: usize, iy: usize, iz: usize, offset:(i32,i32,i32)) -> usize {
        let (dx,dy,dz) = offset;
        let nx = (ix as i32 + dx).rem_euclid(self.num_cells[0] as i32) as usize;
        let ny = (iy as i32 + dy).rem_euclid(self.num_cells[1] as i32) as usize;
        let nz = (iz as i32 + dz).rem_euclid(self.num_cells[2] as i32) as usize;
        
        self.get_1d_idx(nx, ny, nz)
    }

    /// Used during the binning phase
    pub fn get_cell_idx_from_pos(&self, pos: DVec3) -> usize {
        let (ix, iy, iz) = self.get_3d_cell_idx(pos);
        self.get_1d_idx(ix, iy, iz)
    }
}




