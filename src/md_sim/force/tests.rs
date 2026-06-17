

use glam::DVec3;
use crate::md_sim::SimulationSettings;
use crate::md_sim::force::add_weeks_chandler_andersen;
use crate::md_sim::particle::{ActiveParams, CollisionParams, SimulationModel};
use crate::md_sim::utils::{create_particle_vec,create_molecule_vec};

use super::{add_weight,add_viscous_drag, add_granular_collision, add_coulomb};
use super::utils::check_delta;
use super::CellGrid;
use std::f64::consts::PI;

// -----------------------------------------------------------------
// Test single particle forces
// -----------------------------------------------------------------


#[test]
fn test_add_weight() {
    let particles = create_particle_vec();
    let mut forces = vec![DVec3::ZERO; particles.len()];
    
    // Apply weight to the first particle
    add_weight(0, &mut forces, &particles);

    // Assuming gravity is -9.81 and mass is 1.0 (mass = 1.0)
    // Force should be exactly -9.81 in the Z direction
    assert!((forces[0].z + 9.81).abs() < 1e-6);
    
    // Ensure the second particle hasn't been touched
    assert_eq!(forces[1], DVec3::ZERO);
}

#[test]
fn test_add_drag() {
    use std::f64::consts::PI;
    
    let particles = create_particle_vec();
    let mut forces = vec![DVec3::ZERO; particles.len()];
    let viscosity = 0.1;

    // Apply drag to the first particle
    add_viscous_drag(0, &mut forces, &particles, viscosity);
    
    // Expected: -6 * PI * eta * r * v
    // Assuming create_particle_vec sets radius=0.5 and velocity.x=1.0 for particle 0
    let expected_drag_x = -6.0 * PI * viscosity * 0.5 * 1.0;
    
    assert!((forces[0].x - expected_drag_x).abs() < 1e-10);
    
    // Ensure the second particle remains at zero force
    assert_eq!(forces[1], DVec3::ZERO);
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
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
        active_mask: [true; 32]
    };

    let mut forces = vec![DVec3::ZERO; particles.len()];
    let mut torques = vec![DVec3::ZERO; particles.len()];

    // Create a controlled overlap (Combined rad = 1.0, distance = 0.8, overlap = 0.2)
    let mut particles = particles; 
    particles.position[0] = DVec3::ZERO;
    particles.position[1] = DVec3::new(0.8, 0.0, 0.0);

    // --- Case A: Compression (Moving towards each other) ---
    particles.velocity[0] = DVec3::new(1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(-1.0, 0.0, 0.0);

    add_granular_collision(0, 1, &particles, &mut forces, &mut torques, &settings);

    assert!(forces[0].x < 0.0, "Force should be repulsive for particle 0");
    let force_with_damping = forces[0].length();

    // --- Case B: Restitution (Moving away) ---
    forces = vec![DVec3::ZERO; particles.len()]; // Reset force buffer
    particles.velocity[0] = DVec3::new(-1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(1.0, 0.0, 0.0);

    add_granular_collision(0, 1, &particles, &mut forces,&mut torques, &settings);
    let force_no_damping = forces[0].length();

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
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
        active_mask: [true; 32]
    };

    let mut forces = vec![DVec3::ZERO; particles.len()];

    add_weeks_chandler_andersen(0, 1, &mut forces, &particles,  &settings);
    let calc_force = forces[0];

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
        cutoff: 2.0,           // Ensure this is large enough for the overlap
        skin:0.2,
        start: 0,
        num_steps: 100,
        dump: 10,
        interaction_ptypes:vec![[0 as u8,0 as u8]],
        model,                 
        active_mask: [true; 32]
    };

    let mut forces = vec![DVec3::ZERO; particles.len()];

    add_coulomb(0, 1, &particles, &mut forces, &settings);

    const EPS0: f64 = 8.85418782e-12;
    let separation = particles.position[0]-particles.position[1];
    //forces the right are positive
    let coulomb_force = -(1.0/(4.0*PI*EPS0))*-1.0*1.0/separation.length_squared();
    
    assert_eq!(forces[0].length(), coulomb_force);

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
    
    // Case 1: X is far apart (0.9L), should wrap to a small negative distance (-0.1L)
    // Example: Particle A at 0.5, Particle B at 9.5. Delta = 9.0
    let mut delta_x = DVec3::new(9.0, 0.0, 0.0);
    check_delta(&mut delta_x, &sim_box_size);
    assert!((delta_x.x + 1.0).abs() < 1e-6); // 9.0 - 10.0 = -1.0

    // Case 2: Y is negative and far apart, should wrap to a small positive distance
    // Example: Particle A at 9.5, Particle B at 0.5. Delta = -9.0
    let mut delta_y = DVec3::new(0.0, -9.0, 0.0);
    check_delta(&mut delta_y, &sim_box_size);
    assert!((delta_y.y - 1.0).abs() < 1e-6); // -9.0 + 10.0 = 1.0

    // Case 3: Z is already the shortest path, should remain unchanged
    let mut delta_z = DVec3::new(0.0, 0.0, 2.0);
    check_delta(&mut delta_z, &sim_box_size);
    assert!((delta_z.z - 2.0).abs() < 1e-6);
}





//--------------------------------------------------------------------------------------------------
// neighbours tests
// -----------------------------------------------------------------------------------------------


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
        skin: 0.05, // Small skin to ensure we test the "Wide Search"
        ..Default::default()
    };

    // initialise particles
    let mut particles = create_particle_vec();
    println!("{:?}", particles.position);
    // Reposition particles to opposite sides of the X-axis
    // Particle 0 is near the "left" wall
    particles.position[0] = DVec3::new(0.1, 5.0, 5.0);
    // Particle 1 is near the "right" wall
    particles.position[1] = DVec3::new(9.9, 5.0, 5.0);
    println!("{:?}", particles.position);
    println!("{:?}", particles.ref_pos);

    // Initialise the grid
    let mut grid = CellGrid::new(box_size, 2.0, particles.len(), settings.skin);
    // Trigger the build
    // Because ref_pos is still (0,0,0) from the utility, 
    // this will definitely trigger a rebuild.
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    println!("{:?}", particles.position);
    println!("{:?}", particles.ref_pos);
    // Assertions
    // The direct distance is 9.8, but the wrapped distance across the boundary is 0.2.
    // Since 0.2 < (cutoff + skin), they must be neighbours.
    println!("{:?}", grid.verlet_table);
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
    // Simulate the JSON structure for interaction_ptypes = [[0, 0]]
    let interaction_ptypes = vec![[0 as u8, 0 as u8]];

    // Create settings (using a dummy box/cutoff)
    let mut settings = SimulationSettings {
        interaction_ptypes,
        sim_box_size: DVec3::splat(10.0),
        cutoff: 1.0,
        ..Default::default()
    };

    // Trigger the mask building logic from your 'new' function
    // (If this logic is inside SimulationSettings::new, you could also test by 
    // writing a temp JSON file, but testing the loop logic directly is cleaner)
    settings.active_mask = [false; 32];
    for pair in &settings.interaction_ptypes {
        let ptype = pair[0] as usize; // First element defines the searcher
        if ptype < 32 {
            settings.active_mask[ptype] = true;
        }
    }

    assert!(
        settings.is_active(0), 
        "ptype 0 should be active because it appears as pair[0] in [[0,0]]"
    );
    
    assert!(
        !settings.is_active(1), 
        "ptype 1 should NOT be active as it isn't in the interaction list"
    );
}




