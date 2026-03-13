struct CellGrid {
    num_cells: [usize; 3],
    cell_size: f64,
    heads: Vec<Option<usize>>, // Points to the first particle index in each cell
    next: Vec<Option<usize>>,  // Points to the next particle index in the chain
    // Pre-calculate strides for the flattening formula
    stride_y: usize, 
    stride_z: usize,
}


impl CellGrid{

    /// Setup the CellGrid
    pub fn new(box_size: DVec3, cell_size: f64, particle_count: usize) -> Self {
        // Calculate how many cells fit in each dimension
        let nx = (box_size.x / cell_size).floor() as usize;
        let ny = (box_size.y / cell_size).floor() as usize;
        let nz = (box_size.z / cell_size).floor() as usize;

        let total_cells = nx * ny * nz;

        Self {
            num_cells: [nx, ny, nz],
            cell_size,
        
            // Preallocate
            heads: vec![None; total_cells], 
            next: vec![None; particle_count],
            stride_y: nx,
            stride_z: nx * ny,
        }
    }

    /// Bin particles into a particular bin based on their position
    /// 
    /// First calculate the 3d coord of the bin from particle position.
    /// Then add to a linked list for each cell.
    /// The heads array has one position per cell.
    /// The next array is same length as ParticleVec.
    fn bin(&mut self, particles: &ParticleVec){
        self.heads.fill(None);
        
        for (i, pos) in &mut particles.position.iter().enumerate(){
            let cell_coords = get_3d_cell_idx(*pos);
            let cell_idx = self.get_1d_idx(cell_coords);
            self.next[i] = self.heads[cell_idx];
            self.heads[cell_idx] = Some(i);
        }
    }

    /// Search cell grid
    fn search_cell_grid(){
        for ix in 0..&self.num_cells[0]{
            for iy in 0..&self.num_cells[1]{
                for iz in 0..&self.num_cells[2]{
                    cell_idx = get_1d_idx((ix,iy,iz));

                    // If cell is empty this assigns None otherwise Some(particle_id)
                    let mut i_opt = self.heads[cell_idx];                    
                    while let Some(mut i) = self.heads[cell_idx]{ 
                        let mut j_opt = self.next[i];

                        while let Some(mut j) == j_opt{
                            let pos_i = ParticleVec[i].position;
                            let pos_j = ParticleVec[j].position;
                            //Do a test
                            j_opt = self.next[j];
                        }
                        i_opt = self.next[i];
                    }
                }
            }
        }
    }

    /// Find the id of the cell 
    fn get_1d_idx(&self, cell_coord: (usize, usize, usize))->usize{
        let (ix, iy, iz) = cell_coord;
    
        id = ix + iy * self.stride_y + iz * self.stride_z;
        id as usize
    }

    /// Find the cell in the simulation grid to which a particle belongs
    fn get_3d_cell_idx(&self, pos: DVec3)->(usize, usize, usize){
        let x_cells = self.num_cells[0];
        let y_cells = self.num_cells[1];
        let z_cells = self.num_cells[2];

        let idx = ((pos.x/self.cell_size).floor() as usize) % x_cells;
        let idy = ((pos.y/self.cell_size).floor() as usize) % y_cells;
        let idz = ((pos.z/self.cell_size).floor() as usize) % z_cells;

        (idx, idy, idz)
    }
}
