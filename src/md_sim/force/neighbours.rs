use glam::DVec3;
use rayon::prelude::*;

use crate::md_sim::particle::ParticleVec;
use crate::md_sim::SimulationSettings;
use crate::md_sim::utils::{check_delta, InteractionContext};
use crate::md_sim::Forces;

/// A spatial hashing cell grid and Verlet list manager for accelerated pair-force calculations.
#[derive(Debug, Clone)]
pub struct CellGrid {
    // Grid parameters
    pub num_cells: [usize; 3],
    pub cell_size: f64,
    pub inv_cell_size: f64,
    pub sim_box_size: DVec3,
    pub strides: [usize; 3], 
    pub periodic: [bool; 3],
    pub neighbour_table: Vec<Vec<usize>>,

    // Cell CSR storage
    pub cell_offsets: Vec<usize>,       // Length: num_cells + 1
    pub cell_particle_ids: Vec<usize>,  // Length: num_particles   

    // Verlet CSR storage
    pub skin: f64,
    pub verlet_offsets: Vec<usize>,      // Length: num_particles + 1
    pub verlet_particle_ids: Vec<usize>, // Length: total pairs of interactions
    pub counts: Vec<usize>,              // Length: num_particles
    pub last_particle_count: usize,

    // Precomputed interaction rules & matrix
    pub int_context: InteractionContext,
}

impl CellGrid {
    //----------------------------------------------------------------------
    // Public API of CellGrid
    //---------------------------------------------------------------------- 

    /// Creates and initializes a new cell grid spatial partitioning structure.
    pub fn new(particle_count: usize, settings: &SimulationSettings) -> Self {
        let int_context = InteractionContext::new(settings);

        let cell_size = int_context.max_cutoff + settings.skin;
        let inv_cell_size = 1.0 / cell_size;
        let skin = settings.skin;
        let sim_box_size = settings.sim_box_size;
        
        let nx = ((sim_box_size.x * inv_cell_size).floor() as usize).max(1);
        let ny = ((sim_box_size.y * inv_cell_size).floor() as usize).max(1);
        let nz = ((sim_box_size.z * inv_cell_size).floor() as usize).max(1);
        let total_cells = nx * ny * nz;
        let periodic = settings.periodic;
        
        let cell_offsets = vec![0; total_cells + 1];  
        let cell_particle_ids = Vec::with_capacity(particle_count);

        let counts = vec![0; particle_count];
        let verlet_offsets = vec![0; particle_count + 1];  
        let verlet_particle_ids = Vec::with_capacity(12 * particle_count);

        let mut grid = Self {
            num_cells: [nx, ny, nz],
            cell_size,
            inv_cell_size,
            sim_box_size,
            strides: [1, nx, nx * ny],
            periodic,
            neighbour_table: vec![Vec::new(); total_cells],
            cell_offsets,
            cell_particle_ids,
            verlet_offsets,      
            verlet_particle_ids,      
            counts,             
            skin,
            last_particle_count: particle_count, 
            int_context,
        };

        grid.build_neighbour_table();
        grid
    }

    /// Initializes the cell grid and builds initial Verlet lists upon simulation startup.
    pub fn init(&mut self, particles: &mut ParticleVec) {
        self.bin(particles);
        particles.ref_pos.copy_from_slice(&particles.position);
        self.rebuild_verlet_table(particles);
    }

    /// Checks particle displacement and updates neighbour lists if necessary.
    pub fn check_and_rebuild_neighbours(
        &mut self,
        particles: &mut ParticleVec,
        settings: &SimulationSettings,
    ) {
        let threshold_sq = (settings.skin * 0.5).powi(2);

        let count_changed = particles.len() != self.last_particle_count;
        
        let moved_too_far = particles.position.iter()
            .zip(particles.ref_pos.iter())
            .any(|(p, r)| {
                let mut delta = *p - *r;
                check_delta(&mut delta, self.sim_box_size, self.periodic);
                delta.length_squared() > threshold_sq
            });

        if count_changed || moved_too_far {
            if count_changed {
                self.resize_buffers(particles.len());
            }
            
            self.bin(particles);
            self.rebuild_verlet_table(particles);
            particles.ref_pos.copy_from_slice(&particles.position);
            
            self.last_particle_count = particles.len();
        }
    }

    /// Computes all pairwise interactions in parallel using the Verlet neighbor lists.
    pub fn apply_pair_forces<F: Forces + Sync>(
        &self,
        f_buf: &mut [DVec3],
        t_buf: &mut [DVec3],
        particles: &ParticleVec,
        user_impl: &F,
        settings: &SimulationSettings,
    ) {
        let process_particle = |(i, (f_out, t_out)): (usize, (&mut DVec3, &mut DVec3))| {
            let mut local_force = DVec3::ZERO;
            let mut local_torque = DVec3::ZERO;

            let start = self.verlet_offsets[i];
            let end = self.verlet_offsets[i + 1];
            
            for &j in &self.verlet_particle_ids[start..end] {
                let (f, t) = user_impl.update_pair_forces(
                    i, j, DVec3::ZERO, DVec3::ZERO, particles, settings
                );
                local_force += f;
                local_torque += t;
            }

            *f_out += local_force;
            *t_out += local_torque;
        };

        if settings.parallel {
            f_buf.par_iter_mut()
                .zip(t_buf.par_iter_mut())
                .enumerate()
                .for_each(process_particle);
        } else {
            f_buf.iter_mut()
                .zip(t_buf.iter_mut())
                .enumerate()
                .for_each(process_particle);
        }
    }

    //-----------------------------------------------------------------------------------
    // Grid Partitioning
    //-----------------------------------------------------------------------------------

    pub(super) fn build_neighbour_table(&mut self) {
        const OFFSETS: [[i32; 3]; 26] = [
            [1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1],
            [1, 1, 0], [1, -1, 0], [-1, 1, 0], [-1, -1, 0],                    
            [1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1],                    
            [0, 1, 1], [0, 1, -1], [0, -1, 1], [0, -1, -1],                    
            [1, 1, 1], [1, 1, -1], [1, -1, 1], [1, -1, -1],                    
            [-1, 1, 1], [-1, 1, -1], [-1, -1, 1], [-1, -1, -1]                  
        ];

        let (nx, ny, nz) = (self.num_cells[0], self.num_cells[1], self.num_cells[2]);
        self.neighbour_table = vec![Vec::new(); nx * ny * nz];

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let current_1d = self.get_1d_idx(ix, iy, iz);
                    let mut unique_neighbors = Vec::with_capacity(26);
                    
                    for offset in OFFSETS {
                        let n_idx = self.get_neighbour_1d_idx(ix, iy, iz, offset);
                        if n_idx != usize::MAX && n_idx != current_1d && !unique_neighbors.contains(&n_idx) {
                            unique_neighbors.push(n_idx);
                        }
                    }
                    self.neighbour_table[current_1d] = unique_neighbors;
                }
            }
        }
    }

    pub(super) fn bin(&mut self, particles: &ParticleVec) {
        let mut cell_counts = vec![0; self.num_cells[0] * self.num_cells[1] * self.num_cells[2]];
        
        for pos in particles.position.iter() {
            let cell_idx = self.get_cell_idx_from_pos(pos); 
            cell_counts[cell_idx] += 1;
        }

        self.cell_offsets[0] = 0;
        for i in 0..cell_counts.len() {
            self.cell_offsets[i + 1] = self.cell_offsets[i] + cell_counts[i];
        }

        let mut current_pos = self.cell_offsets.clone(); 
        self.cell_particle_ids.resize(particles.position.len(), 0);

        for (i, pos) in particles.position.iter().enumerate() {
            let cell_idx = self.get_cell_idx_from_pos(pos);
            let target_idx = current_pos[cell_idx];
            self.cell_particle_ids[target_idx] = i;
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

    #[inline(always)]
    pub(super) fn get_1d_idx(&self, ix: usize, iy: usize, iz: usize) -> usize {
        ix + iy * self.strides[1] + iz * self.strides[2]
    }

    #[inline(always)]
    pub(super) fn get_neighbour_1d_idx(&self, ix: usize, iy: usize, iz: usize, offsets: [i32; 3]) -> usize {
        let mut coords = [ix as i32, iy as i32, iz as i32];

        for i in 0..3 {
            let val = coords[i] + offsets[i];
            
            if self.periodic[i] {
                coords[i] = val.rem_euclid(self.num_cells[i] as i32);
            } else {
                if val < 0 || val >= self.num_cells[i] as i32 {
                    return usize::MAX; 
                }
                coords[i] = val;
            };
        }
        self.get_1d_idx(coords[0] as usize, coords[1] as usize, coords[2] as usize)
    }

    fn resize_buffers(&mut self, particle_count: usize) {
        self.counts.resize(particle_count, 0);
        self.verlet_offsets.resize(particle_count + 1, 0);
        self.verlet_particle_ids.clear();
    }

    //------------------------------------------------------------------------------------
    // Verlet List Construction
    //------------------------------------------------------------------------------------
    
    pub(super) fn rebuild_verlet_table(&mut self, particles: &ParticleVec) {
        let offsets = &self.cell_offsets;
        let indices = &self.cell_particle_ids;
        let neighbours = &self.neighbour_table;

        self.counts.fill(0);

        // --- PASS 1: Count ---
        for cell_idx in 0..offsets.len() - 1 {
            let range = offsets[cell_idx]..offsets[cell_idx + 1];
            for &i in &indices[range.clone()] {
                for &j in &indices[range.clone()] {
                    if self.add_to_verlet(i, j, particles) {
                        self.counts[i] += 1;
                    }
                }
                for &n_idx in &neighbours[cell_idx] {
                    let n_range = offsets[n_idx]..offsets[n_idx + 1];
                    for &j in &indices[n_range] {
                        if self.add_to_verlet(i, j, particles) {
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
        
        let total_pairs = self.verlet_offsets[particles.position.len()];
        self.verlet_particle_ids.resize(total_pairs, 0);

        // --- PASS 3: Fill ---
        let mut current_pos = self.verlet_offsets.clone();
        for cell_idx in 0..offsets.len() - 1 {
            let range = offsets[cell_idx]..offsets[cell_idx + 1];
            for &i in &indices[range.clone()] {
                for &j in &indices[range.clone()] {
                    if self.add_to_verlet(i, j, particles) {
                        self.verlet_particle_ids[current_pos[i]] = j;
                        current_pos[i] += 1;
                    }
                }
                for &n_idx in &neighbours[cell_idx] {
                    let n_range = offsets[n_idx]..offsets[n_idx + 1];
                    for &j in &indices[n_range] {
                        if self.add_to_verlet(i, j, particles) {
                            self.verlet_particle_ids[current_pos[i]] = j;
                            current_pos[i] += 1;
                        }
                    }
                }
            }
        }
    }

    #[inline(always)]
    pub(super) fn add_to_verlet(&self, i: usize, j: usize, p: &ParticleVec) -> bool {
        if p.molecule_id[i] == p.molecule_id[j] { return false; }

        let ptype_i = p.ptype[i] as usize;
        let ptype_j = p.ptype[j] as usize;

        let search_radius_sq = self.int_context.search_radius_sq_matrix[ptype_i][ptype_j];
        if search_radius_sq == 0.0 { return false; }

        let mut delta = p.position[i] - p.position[j];
        check_delta(&mut delta, self.int_context.sim_box_size, self.int_context.periodic);
        
        delta.length_squared() < search_radius_sq
    }
}
