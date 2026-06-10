use std::collections::HashMap;

use glam::DVec3;
use crate::md_sim::particle::ParticleVec;
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
    pub heads: Vec<Option<usize>>, 
    pub next: Vec<Option<usize>>,  
    pub stride_y: usize, 
    pub stride_z: usize,
    pub neighbour_table: Vec<Vec<usize>>,
    // For the verlet lists
    pub verlet_table: Vec<Vec<usize>>,
    pub skin: f64,
    pub last_particle_count: usize,
}

impl CellGrid {
    pub fn new(box_size: DVec3, cell_size: f64, particle_count: usize, skin: f64) -> Self {
        let nx = ((box_size.x / cell_size).floor() as usize).max(1);
        let ny = ((box_size.y / cell_size).floor() as usize).max(1);
        let nz = ((box_size.z / cell_size).floor() as usize).max(1);
        let total_cells = nx * ny * nz;

        // All 26 possible neighbour directions in a 3D grid
        const OFFSETS: [(i32, i32, i32); 26] = [
            (1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1), // Faces
            (1, 1, 0), (1, -1, 0), (-1, 1, 0), (-1, -1, 0),                     // Edges XY
            (1, 0, 1), (1, 0, -1), (-1, 0, 1), (-1, 0, -1),                     // Edges XZ
            (0, 1, 1), (0, 1, -1), (0, -1, 1), (0, -1, -1),                     // Edges YZ
            (1, 1, 1), (1, 1, -1), (1, -1, 1), (1, -1, -1),                     // Corners
            (-1, 1, 1), (-1, 1, -1), (-1, -1, 1), (-1, -1, -1)                  // Corners
        ];


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

        // Populate the neighbour_table (Broad-phase map)
        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let current_1d = grid.get_1d_idx(ix, iy, iz);
                    let mut unique_neighbours = Vec::new();
                    for offset in OFFSETS {
                        let n_idx = grid.get_wrapped_1d_idx(ix, iy, iz, offset);
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

    /// check and rebuild neighbours
    /// 
    /// This checks whether any particle has moved more than the skin/2. If so then the cell grid and verlet lists
    /// are rebuilt. If the number of particles has changed we also rebuild everything.
    pub fn check_and_rebuild_neighbours(&mut self,particles: &mut ParticleVec,settings: &SimulationSettings) {
        let threshold_sq = (settings.skin * 0.5).powi(2);

        // Check if the number of particles has changed
        let count_changed = particles.len() != self.last_particle_count;

        // Check if any particle has moved too far
        let moved_too_far = if !count_changed {
            particles.position.iter()
                .zip(particles.ref_pos.iter())
                .any(|(p, r)| {
                    let mut delta = *p - *r;
                    check_delta(&mut delta, &settings.sim_box_size);
                    delta.length_squared() > threshold_sq
                })
        } else {
            true // Always rebuild if count changed
        };

        if count_changed || moved_too_far {
            // Ensure our internal buffers (next, verlet_table) match the new size
            if count_changed {
                self.resize_buffers(particles.len());
            }
            
            self.bin(particles);
            self.rebuild_verlet_table(particles, settings);
            
            // Update the count tracker
            self.last_particle_count = particles.len();
        }
    }

    fn resize_buffers(&mut self, new_count: usize) {
        self.next.resize(new_count, None);
        self.verlet_table.resize(new_count, Vec::with_capacity(20));
    }

    // Put particles into cells and build linked lists associated with each cell
    fn bin(&mut self, particles: &ParticleVec) {
        self.heads.fill(None);
        for (i, pos) in particles.position.iter().enumerate() {
            let (ix, iy, iz) = self.get_3d_cell_idx(*pos);
            let cell_idx = self.get_1d_idx(ix, iy, iz);
            self.next[i] = self.heads[cell_idx];
            self.heads[cell_idx] = Some(i);
        }
    }

    fn rebuild_verlet_table(&mut self, particles: &mut ParticleVec, settings: &SimulationSettings) {
        let search_radius = settings.cutoff + self.skin;
        let search_radius_sq = search_radius * search_radius;

        for list in &mut self.verlet_table {
            list.clear();
        }

        // Sync the reference positions
        particles.ref_pos.copy_from_slice(&particles.position);

        //for each cell in the grid look at other particles in the linked list
        // and check if they should be added to that particles verlet list. Then repeat
        // process for particles in the neighbouring cells. The neighbouring cell
        // might be wrapped since sim box boundaries are periodic.
        for cell_idx in 0..self.heads.len() {
            let mut i_opt = self.heads[cell_idx];
            while let Some(i) = i_opt {
                // Look at same cell
                let mut j_opt = self.heads[cell_idx]; 
                while let Some(j) = j_opt {
                    self.try_add_pair(i, j, search_radius_sq, particles, settings);
                    j_opt = self.next[j];
                }

                // Look at neighbour cells
                for neighbour_idx in self.neighbour_table[cell_idx].clone() {
                    let mut nj_opt = self.heads[neighbour_idx];
                    while let Some(j) = nj_opt {
                        self.try_add_pair(i, j, search_radius_sq, particles, settings);
                        nj_opt = self.next[j];
                    }
                }
                i_opt = self.next[i];
                }
            }
        }

    // Check if pair should be added to particles verlet list.
    fn try_add_pair(&mut self, i: usize, j: usize, r_sq: f64, p: &ParticleVec, settings: &SimulationSettings) {
        // This prevents self-interaction (i == j) and double counting
        if i >= j {
            return;
        }

        // determine if particles are active
        let ptype_i = p.ptype[i];
        let ptype_j = p.ptype[j];

        // Check if this pair is allowed to interact based on the JSON config
        let is_pair_allowed = settings.interaction_ptypes.iter()
            .any(|pair| (pair[0] == ptype_i as u8 && pair[1] == ptype_j as u8) || 
                    (pair[0] == ptype_j as u8 && pair[1] == ptype_i as u8));

        if !is_pair_allowed {
            return;
        }

        let active_i = settings.is_active(ptype_i);
        let active_j = settings.is_active(ptype_j);

        // If both are static ignore
        if !active_i && !active_j { return; }

        // Calculate wrapped distance
        let mut delta = p.position[i] - p.position[j];
        check_delta(&mut delta, &settings.sim_box_size);
        
        if delta.length_squared() < r_sq {
            // If i active add to j verlet list
            if active_i{
                self.verlet_table[i].push(j);
            }

            // If j active add to i verlet list
            if active_j{
                self.verlet_table[j].push(i);
            }
        }

        
    }

    /// loop that uses the verlet_table
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
                check_delta(&mut delta, &settings.sim_box_size);
                
                let dist_sq = delta.length_squared();
                if dist_sq < cutoff_sq {
                    user_impl.update_pair_forces(i, j, f_buf, t_buf, particles, settings);
                }
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_particle_vec;

    #[test]
    fn test_first_frame_rebuild() {
        let box_size = DVec3::splat(10.0);
        let settings = SimulationSettings {
            sim_box_size: box_size,
            cutoff: 2.0, // Increased cutoff to be safe
            skin: 0.2,
            ..Default::default()
        };

        let mut particles = create_particle_vec();
        
        // Place them very close together
        particles.position[0] = DVec3::new(1.0, 1.0, 1.0);
        particles.position[1] = DVec3::new(1.1, 1.1, 1.1);
        particles.ref_pos.copy_from_slice(&particles.position);

        let mut grid = CellGrid::new(box_size, 2.0, particles.len(), settings.skin);

        // Move particle 0 past the threshold (0.15 > 0.1)
        particles.position[0] += DVec3::new(0.15, 0.0, 0.0);

        grid.check_and_rebuild_neighbours(&mut particles, &settings);

        assert_eq!(particles.ref_pos[0], particles.position[0]);
        assert!(grid.verlet_table[0].contains(&1));
    }


    #[test]
    fn test_skin_displacement_trigger() {
        // Setup
        let box_size = DVec3::splat(10.0);
        let settings = SimulationSettings {
            sim_box_size: box_size,
            cutoff: 1.0,
            skin: 0.4, // Displacement threshold is skin * 0.5 = 0.2
            ..Default::default()
        };

        // initialise particles
        let mut particles = create_particle_vec();//p1.pos and p2.pos = (1.0,2.0,3.0), p1.vel = (1.0, 1.0, 1.0), p2.vel = (0.1, 0.2, 0.3)
        particles.ref_pos.copy_from_slice(&particles.position);

        let mut grid = CellGrid::new(box_size, settings.cutoff + settings.skin, particles.len(), settings.skin);

        // PRIME THE GRID: This sets last_particle_count and syncs ref_pos
        grid.check_and_rebuild_neighbours(&mut particles, &settings);

        // Move particle[0].x slightly (0.1 units)
        // 0.1 < skin/2 threshold -> Should NOT rebuild
        particles.position[0] += DVec3::new(0.1, 0.0, 0.0);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);
        
        assert_ne!(
            particles.ref_pos[0], 
            particles.position[0], 
            "ref_pos should still be the old position (no rebuild yet)."
        );

        // Move particle 0 further (another 0.2 units, total 0.3)
        // 0.3 > 0.2 threshold -> Should trigger a rebuild
        particles.position[0] += DVec3::new(0.2, 0.0, 0.0);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);
        
        assert_eq!(
            particles.ref_pos[0], 
            particles.position[0], 
            "ref_pos should now match position because a rebuild was triggered."
        );
    }

    #[test]
    fn test_periodic_neighbours() {
        // Setup
        let box_size = DVec3::splat(10.0);
        let settings = SimulationSettings {
            sim_box_size: box_size,
            cutoff: 1.5,
            skin: 0.1, // Small skin to ensure we test the "Wide Search"
            ..Default::default()
        };

        // initialise particles
        let mut particles = create_particle_vec();
        
        // Reposition particles to opposite sides of the X-axis
        // Particle 0 is near the "left" wall
        particles.position[0] = DVec3::new(0.1, 5.0, 5.0);
        // Particle 1 is near the "right" wall
        particles.position[1] = DVec3::new(9.9, 5.0, 5.0);

        // Initialise the grid
        let mut grid = CellGrid::new(box_size, 2.0, particles.len(), settings.skin);
        
        // Trigger the build
        // Because ref_pos is still (0,0,0) from the utility, 
        // this will definitely trigger a rebuild.
        grid.check_and_rebuild_neighbours(&mut particles, &settings);

        // Assertions
        // The direct distance is 9.8, but the wrapped distance across the boundary is 0.2.
        // Since 0.2 < (cutoff + skin), they must be neighbours.
        assert!(
            grid.verlet_table[0].contains(&1), 
            "Particles should be identified as neighbours across the periodic boundary."
        );
        
        // Verify that the table only contains the pair once (i < j logic)
        assert_eq!(grid.verlet_table[0].len(), 1);
        
    }

    #[test]
    fn test_active_ghost_interaction() {
        let box_size = DVec3::splat(10.0);
        // Setup: Type 0 is active, Type 1 is a ghost (not in active_mask)
        let mut settings = SimulationSettings {
            sim_box_size: box_size,
            cutoff: 1.0,
            skin: 0.2,
            active_mask: [false; 32],
            ..Default::default()
        };
        settings.active_mask[0] = true; // Only 0 is active

        let mut particles = create_particle_vec();
        particles.ptype[0] = 0; // Ball
        particles.ptype[1] = 1; // Floor
        particles.position[0] = DVec3::new(5.0, 5.0, 5.0);
        particles.position[1] = DVec3::new(5.0, 5.0, 5.5); // 0.5 distance

        let mut grid = CellGrid::new(box_size, 1.2, particles.len(), 0.2);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);

        // Ball (0) should have Floor (1) in its list because 0 is active
        assert!(grid.verlet_table[0].contains(&1), "Active particle should see the ghost particle");
        
        // Floor (1) should NOT have Ball (0) in its list because 1 is inactive
        assert!(!grid.verlet_table[1].contains(&0), "Ghost particle should not have its own verlet list populated");
    }


    #[test]
    fn test_ghost_ghost_invisibility() {
        let box_size = DVec3::splat(10.0);
        let settings = SimulationSettings {
            sim_box_size: box_size,
            cutoff: 1.0,
            skin: 0.2,
            active_mask: [false; 32],
            ..Default::default()
        };
        // Neither 1 nor 2 are active

        let mut particles = create_particle_vec();
        particles.ptype[0] = 1; 
        particles.ptype[1] = 2; 
        particles.position[0] = DVec3::new(5.0, 5.0, 5.0);
        particles.position[1] = DVec3::new(5.0, 5.0, 5.2);

        let mut grid = CellGrid::new(box_size, 1.2, particles.len(), 0.2);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);

        assert!(grid.verlet_table[0].is_empty());
        assert!(grid.verlet_table[1].is_empty());
    }

    #[test]
    fn test_active_mask_derivation_from_json_logic() {
        // 1. Simulate the JSON structure for interaction_ptypes = [[0, 0]]
        let interaction_ptypes = vec![[0 as u8, 0 as u8]];

        // 2. Create settings (using a dummy box/cutoff)
        let mut settings = SimulationSettings {
            interaction_ptypes,
            sim_box_size: DVec3::splat(10.0),
            cutoff: 1.0,
            ..Default::default()
        };

        // 3. Manually trigger the mask building logic from your 'new' function
        // (If this logic is inside SimulationSettings::new, you could also test by 
        // writing a temp JSON file, but testing the loop logic directly is cleaner)
        settings.active_mask = [false; 32];
        for pair in &settings.interaction_ptypes {
            let ptype = pair[0] as usize; // First element defines the searcher
            if ptype < 32 {
                settings.active_mask[ptype] = true;
            }
        }

        // 4. Assertions
        assert!(
            settings.is_active(0), 
            "ptype 0 should be active because it appears as pair[0] in [[0,0]]"
        );
        
        assert!(
            !settings.is_active(1), 
            "ptype 1 should NOT be active as it isn't in the interaction list"
        );
    }
}

