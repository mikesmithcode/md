

use glam::DVec3;

use crate::md_sim::SimulationSettings;
use crate::md_sim::force::add_weeks_chandler_andersen;
use crate::md_sim::particle::{ActiveParams, CollisionParams, SimulationModel};
use crate::md_sim::utils::{create_particle_vec,create_molecule_vec, create_grid_and_settings};

use super::{add_weight,add_viscous_drag, add_granular_collision, add_coulomb};
use super::utils::check_delta;
use std::f64::consts::PI;

// -----------------------------------------------------------------
// Test single particle forces
// -----------------------------------------------------------------


#[test]
fn test_add_weight() {
    let particles = create_particle_vec();
    let mut force = DVec3::ZERO;
    
    // Apply weight to the first particle
    force = add_weight(0, force, &particles);

    // Assuming gravity is -9.81 and mass is 1.0 (mass = 1.0)
    // Force should be exactly -9.81 in the Z direction
    assert!((force.z + 9.81).abs() < 1e-6);

}

#[test]
fn test_add_drag() {
    use std::f64::consts::PI;
    
    let particles = create_particle_vec();
    let mut force = DVec3::ZERO;
    let viscosity = 0.1;

    // Apply drag to the first particle
    force = add_viscous_drag(0, &particles,force, viscosity);
    
    // Expected: -6 * PI * eta * r * v
    // Assuming create_particle_vec sets radius=0.5 and velocity.x=1.0 for particle 0
    let expected_drag_x = -6.0 * PI * viscosity * 0.5 * 1.0;
    
    assert!((force.x - expected_drag_x).abs() < 1e-10);
    

}

fn test_add_active_force(){
    assert!(false, "Need test for add_active_force");
}


// -----------------------------------------------------------------
// Test pair particle forces
// -----------------------------------------------------------------

#[test]
fn test_granular_collision() {
    let particles = create_particle_vec();
    
    // Bundle params into the specific Enum variant
    let model = SimulationModel::Solid(CollisionParams {
        stiffness: 1000.0,
        damping: 50.0,
    });

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        periodic: [true;3],
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
    };

    let mut force = DVec3::ZERO;

    // Create a controlled overlap (Combined rad = 1.0, distance = 0.8, overlap = 0.2)
    let mut particles = particles; 
    particles.position[0] = DVec3::ZERO;
    particles.position[1] = DVec3::new(0.8, 0.0, 0.0);

    // --- Case A: Compression (Moving towards each other) ---
    particles.velocity[0] = DVec3::new(1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(-1.0, 0.0, 0.0);

    (force, _) = add_granular_collision(0, 1, &particles, force, DVec3::ZERO, &settings);

    assert!(force.x < 0.0, "Force should be repulsive for particle 0");
    let force_with_damping = force.length();

    // --- Case B: Restitution (Moving away) ---
    force = DVec3::ZERO; // Reset force buffer
    particles.velocity[0] = DVec3::new(-1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(1.0, 0.0, 0.0);

    (force, _ )=add_granular_collision(0, 1, &particles, force,DVec3::ZERO, &settings);
    let force_no_damping = force.length();

    // force_with_damping (Compression) should be > force_no_damping (Restitution).
    assert!(force_with_damping > force_no_damping, "Damping must increase total force magnitude during compression");
}

#[test]
fn test_weeks_chandler_andersen() {
    let particles = create_molecule_vec();
    //particles are both 0.5 radius
    
    // Bundle params into the specific Enum variant
    let model = SimulationModel::Active(ActiveParams {
    stiffness: 100.0,
    damping: 1.0,
    Dt: 0.1,
    v0: 1.0,
    gamma: 1.0});

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        periodic: [true;3],
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
    };

    let mut force = DVec3::ZERO;

    force = add_weeks_chandler_andersen(0, 1, &particles, force, &settings);
    let calc_force = force;

    let epsilon: f64 = 100.0;
    let sigma: f64 = 1.0;
    let r: DVec3=particles.position[0]-particles.position[1];
    let r2:f64 = r.length_squared();
    let f_expected = r*(48.0 * epsilon / r2) *  (sigma.powi(12)/(r2.powi(6)) - 0.5 * (sigma.powi(6)/r2.powi(3))) / r2.sqrt() ;
    
    assert!((f_expected[0] - calc_force[0]).abs() < 0.0000001 , "WCA not giving expected value"); 
}

#[test]
fn test_coulomb() {
    let mut particles = create_molecule_vec();
    
    particles.charge[0] = 1.0;
    particles.charge[1] = -1.0;

    // Bundle params into the specific Enum variant
    let model = SimulationModel::Solid(CollisionParams {
        stiffness: 1000.0,
        damping: 50.0,
    });

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        periodic: [true;3],
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
    };

    let mut force = DVec3::ZERO;

    force = add_coulomb(0, 1, &particles, force, &settings);

    const EPS0: f64 = 8.85418782e-12;
    let separation = particles.position[0]-particles.position[1];
    //forces the right are positive
    let coulomb_force = -(1.0/(4.0*PI*EPS0))*-1.0*1.0/separation.length_squared();
    
    assert_eq!(force.length(), coulomb_force);

}

//--------------------------------------------------------------------------------------------------
// bonds tests
// -----------------------------------------------------------------------------------------------



// -----------------------------------------------------------------
// Test utility functions
// -----------------------------------------------------------------

#[test]
fn test_check_delta() {
    let sim_box_size = DVec3::new(10.0, 10.0, 10.0);
    let periodic = [true;3];
    // Case 1: X is far apart (0.9L), should wrap to a small negative distance (-0.1L)
    // Example: Particle A at 0.5, Particle B at 9.5. Delta = 9.0
    let mut delta_x = DVec3::new(9.0, 0.0, 0.0);
    check_delta(&mut delta_x, sim_box_size, periodic);
    assert!((delta_x.x + 1.0).abs() < 1e-6); // 9.0 - 10.0 = -1.0

    // Case 2: Y is negative and far apart, should wrap to a small positive distance
    // Example: Particle A at 9.5, Particle B at 0.5. Delta = -9.0
    let mut delta_y = DVec3::new(0.0, -9.0, 0.0);
    check_delta(&mut delta_y, sim_box_size, periodic);
    assert!((delta_y.y - 1.0).abs() < 1e-6); // -9.0 + 10.0 = 1.0

    // Case 3: Z is already the shortest path, should remain unchanged
    let mut delta_z = DVec3::new(0.0, 0.0, 2.0);
    check_delta(&mut delta_z, sim_box_size, periodic);
    assert!((delta_z.z - 2.0).abs() < 1e-6);
}





//--------------------------------------------------------------------------------------------------
// neighbours tests
// -----------------------------------------------------------------------------------------------
    // check neighbours in 3x3x3 grid with and without periodic boundary conditions give correct values.
    #[test]
    fn test_build_neighbour_table(){
        
        let (mut grid, _settings) = create_grid_and_settings();
        
        //periodic
        grid.periodic = [true;3];
        grid.build_neighbour_table();

        assert!(grid.neighbour_table.len() == 27, "Should be 27 boxes in grid");
        assert_eq!(grid.neighbour_table[0], [1, 2, 3, 6, 9, 18, 4, 7, 5, 8, 10, 19, 11, 20, 12, 21, 15, 24, 13, 22, 16, 25, 14, 23, 17, 26], "Neighbours incorrect under periodic boundary conditions");

        // non-periodic. 
        grid.periodic = [false;3];
        grid.neighbour_table = vec![[usize::MAX; 26]; 27];
        grid.build_neighbour_table();

    
        assert!(grid.neighbour_table.len() == 27, "Should be 27 boxes in grid");
        let correct_neighbours: Vec<usize> = vec![1, 3, 9, 4, 10, 12, 13];
        assert!(grid.neighbour_table[0].iter().copied().filter(|&x| x!=usize::MAX).collect::<Vec<usize>>() == correct_neighbours, "Should be 7 boxes in non-periodic grid for (0,0,0)");

    }


   #[test]
    fn test_get_1d_idx(){
        let (grid, _settings)=create_grid_and_settings();

        let ix: usize=2;
        let iy: usize=2;
        let iz: usize=2;

        let idx = grid.get_1d_idx(ix,iy,iz);
        assert_eq!(idx, 26, "(2,2,2) should be 26");
    }
    
   #[test]
    fn test_get_neighbour_1d_idx(){
        let (mut grid, _settings)=create_grid_and_settings();

        let ix: usize=0;
        let iy: usize=0;
        let iz: usize=0;

        //test value outside grid in non-periodic results in None
        grid.periodic = [false;3];
        let new_coords = grid.get_neighbour_1d_idx(ix,iy,iz, (-1,0,0));
        assert_eq!(new_coords, None, "coords should have returned None because outside box");

        //test values in periodic box.
        grid.periodic = [true;3];
        grid.neighbour_table = vec![[usize::MAX; 26]; 27];

        let new_coords = grid.get_neighbour_1d_idx(ix,iy,iz, (-1,0,0));
        assert_eq!(new_coords, Some(2) , "x coord should have wrapped");

    }

    #[test]
    fn test_get_3d_cell_idx(){
        let (mut grid, _settings)=create_grid_and_settings();
        grid.periodic = [false;3];
        grid.neighbour_table = vec![[usize::MAX; 26]; 27];

        let illegal_position = DVec3::new(-1.0, 0.0, 0.0);
        let return_val = grid.get_3d_cell_idx(illegal_position);
        assert_eq!(return_val, None, "func should return None if particle outside simulation box");

        let allowed_position = DVec3::new(1.0,1.0,1.0);
        let return_val = grid.get_3d_cell_idx(allowed_position);
        assert_eq!(return_val, Some((0,0,0)), "func should return indices Some((0,0,0))");
    }

    // tests that particles are added to cells and then linked lists correctly created.
    #[test]
    fn test_bin(){
        let (mut grid, _settings)=create_grid_and_settings();
        let particles = create_particle_vec();
        grid.bin(&particles);

        assert_eq!(grid.next, [None, Some(0), None, None, Some(3), None],"the next array has the wrong values");
        assert_eq!(grid.heads, [None, None, None, None, None, None, None, None, None, Some(1), None, None, None, Some(5), None, None, None, Some(4), None, None, None, None, None, None, None, None, Some(2)],"The heads array in the grid is incorrectly populated");
    }

    #[test]
    fn test_try_add_pair_conditions() {
        let (mut grid, settings) = create_grid_and_settings();
        let particles = create_molecule_vec();
        
        // Define allowed interactions based on config
        let interaction_ptypes = vec![[0, 1]]; 
        let search_radius_sq = (settings.cutoff + settings.skin).powi(2);

        // Same molecule_id (should be ignored)
        // Particle 0 and 1 are both molecule_id: 0
        grid.try_add_pair(0, 1, search_radius_sq, &particles, &interaction_ptypes);
        assert!(grid.verlet_table[0].is_empty());

        // Different molecules, valid types, close proximity (should be added)
        // Particle 0 (mol 0, type 0) and Particle 2 (mol 1, type 0) - wait, type mismatch!
        // Try Particle 0 (mol 0, type 0) and Particle 3 (mol 1, type 1)
        grid.try_add_pair(0, 3, search_radius_sq, &particles, &interaction_ptypes);
        assert!(grid.verlet_table[0].contains(&3), "Valid pair should have been added");

        // Different molecules, invalid types (should be ignored)
        // Particle 0 (type 0) and Particle 2 (type 0) - type [0,0] is not in interaction_ptypes
        grid.try_add_pair(0, 2, search_radius_sq, &particles, &interaction_ptypes);
        assert!(!grid.verlet_table[0].contains(&2));
    }





    #[test]
    fn test_resize_buffers(){
        let (mut grid, _settings)=create_grid_and_settings();

        assert!(grid.verlet_table.len()==6);
        grid.resize_buffers(7);
        assert!(grid.verlet_table.len()==7);
    }


    //Verlet tests
    #[test]
    fn test_first_frame_rebuild() {
        let (mut grid, settings) = create_grid_and_settings();
        let mut particles = create_particle_vec();
        
        particles.position[0] = DVec3::new(1.0,1.0,1.0);
        particles.ref_pos[0] = DVec3::new(5.0,5.0,5.0);
    
        grid.init(&mut particles, &settings);

        assert_eq!(particles.ref_pos[0], particles.position[0]);
        // Verify index 0 and 2 are neighbours (based on create_molecule_vec layout)
        assert!(grid.verlet_table[0].contains(&1));
    }

    #[test]
    fn test_skin_displacement_trigger() {
        let (mut grid, settings) = create_grid_and_settings();
        let mut particles = create_molecule_vec();
        

        //pos and ref_pos should be the same
        grid.init(&mut particles, &settings);

        // Move 0.09 (less than skin/2 = 0.1), shouldn't rebuild
        particles.position[0] += DVec3::new(0.09, 0.0, 0.0);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);
        assert_ne!(particles.ref_pos[0], particles.position[0], "Should not have rebuilt");

        // Move another 0.02 (total 0.11 > skin/2 = 0.2/2)
        particles.position[0] += DVec3::new(0.2, 0.0, 0.0);
        grid.check_and_rebuild_neighbours(&mut particles, &settings);
        
        assert_eq!(particles.ref_pos[0], particles.position[0], "Should have triggered rebuild");
    }

    #[test]
    fn test_molecular_exclusion() {
        let (mut grid, _settings) = create_grid_and_settings();
        let particles = create_molecule_vec();
        
        // Particles 0 and 1 belong to molecule 0 so shouldn't be in each other's verlet table
        let i = 0;
        let j = 1;
        
        // Attempt to add a pair that is physically close but within the same molecule
        grid.try_add_pair(i, j, 5.0, &particles, &[[0,0]]);
        
        assert!(grid.verlet_table[i].is_empty(), "Particles in same molecule must be excluded");
    }

    #[test]
    fn test_periodic_neighbours() {
        let (mut grid, settings) = create_grid_and_settings();
        let mut particles = create_particle_vec();
        
        // Place particles across periodic boundary
        particles.position[0] = DVec3::new(0.1, 5.0, 5.0);
        particles.position[1] = DVec3::new(8.9, 5.0, 5.0); // 1.2 distance, within cutoff 3.0
        

        grid.check_and_rebuild_neighbours(&mut particles, &settings);
        
        assert!(grid.verlet_table[0].contains(&1), "Should detect periodic neighbour");
    }

    #[test]
    fn test_ptype_interactions() {
        let (mut grid, settings) = create_grid_and_settings();
        let mut particles = create_particle_vec();
        grid.init(&mut particles, &settings);

        // Ball (id=0) should have ball (id=1) in its list because interaction_ptype = vec![[0,1]]
        assert!(grid.verlet_table[0].contains(&1), "0 should see 1");
        
        // Ball (id=1) should NOT have Ball (id=0) in its list because interaction_ptype not specified.
        assert!(!grid.verlet_table[1].contains(&0), "1 should not see 0");
    }










