

use glam:: DVec3;
use three_d::Srgba;

use crate::md_sim::force::{check_particle_contact, check_object_contact, CollisionParams};
use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{Particle, RectSpec, ObjectSpec, ParticleVec, SurfaceKinematics};
use crate::md_sim::utils::{create_particle_vec,create_molecule_vec, create_grid_and_settings, assert_dvec3_near};
use crate::md_sim::utils::InteractionContext;
use crate::md_sim::force::contact::Contact;
use crate::md_sim::force::{add_gravity,Gravity, add_viscous_drag, ViscousDrag, add_coulomb, CoulombParams};
use crate::md_sim::force::{normal_hertzian, normal_linear, friction_viscous_damping};
use super::neighbours::CellGrid;
use std::f64::consts::PI;

//====================================================================================================================
// Test single particle forces
//====================================================================================================================

///=====================================================================================================================
/// **What:** Verifies that gravitational body forces are correctly calculated and applied.  
/// **How:** Applies weight to the first particle using `add_weight` and inspects the resulting force vector.  
/// **Why:** Ensures that gravitational acceleration maps cleanly to the vertical force buffer for single-body dynamics.
/// ====================================================================================================================
#[test]
fn test_add_gravity() {
    let mut particles = ParticleVec::new();
    particles.push(Particle::default());

    let mut force = DVec3::ZERO;
    let gravity = Gravity::default();
    
    // Apply weight to the first particle
    force = add_gravity(0, force, &particles, &gravity);

    // Assuming gravity is -9.81 and mass is 1.0 
    // Force should be exactly -9.81 in the Z direction
    assert!((force.z + 9.81).abs() < 1e-6);

}

/// ====================================================================================================================
/// **What:** Validates velocity-dependent Stokes' law viscous drag calculations.  
/// **How:** Computes drag force on a particle with known velocity and radius against a specified fluid viscosity.  
/// **Why:** Ensures that drag damping forces scale correctly relative to particle dimensions and surrounding medium parameters.
/// ====================================================================================================================
#[test]
fn test_add_drag() {
    use std::f64::consts::PI;
    
    let mut particles = ParticleVec::new();
    particles.push(Particle::default());
    let mut force = DVec3::ZERO;
    let viscous_drag = ViscousDrag::default();

    // Apply drag to the first particle
    force = add_viscous_drag(0, &particles,force, &viscous_drag);
    
    // Expected: -6 * PI * eta * r * v
    // Assuming create_particle_vec sets radius=0.5 and velocity.x=1.0 for particle 0
    let expected_drag_x = -6.0 * PI * viscous_drag.viscosity * 0.5 * 1.0;
    
    assert!((force.x - expected_drag_x).abs() < 1e-10);
    

}


// ====================================================================================================================
// Test pair particle forces
// ====================================================================================================================


/// ====================================================================================================================
/// **What:** Validates the Hertzian non-linear elastic and viscous damping normal force calculation.  
/// **How:** Computes normal forces for a particle contact with fractional overlap and approaching relative velocity.  
/// **Why:** Ensures that contact elasticity follows the 1.5 power-law scaling and damping adjusts magnitudes correctly during collisions.
/// ====================================================================================================================
#[test]
fn test_normal_hertzian() {
    let contact = Contact {
        overlap: 0.01,
        eff_stiffness: 1e4,
        eff_damping: 10.0,
        rel_vel: DVec3::new(0.0, 0.0, -0.5), // Approaching along normal
        ..Contact::default()
    };

    let (f_mag, f_vec) = normal_hertzian(&contact);
    
    // Expected:
    // f_elastic = 1e4 * (0.01)^1.5 = 10000 * 0.001 = 10.0
    // f_damping = 10.0 * (-0.5 * 1.0) = -5.0
    // f_normal_mag = (10.0 - (-5.0)).max(0.0) = 15.0
    assert!((f_mag - 15.0).abs() < 1e-6);
    assert!((f_vec.z - 15.0).abs() < 1e-6);
}

/// ====================================================================================================================
/// **What:** Validates linear elastic and viscous damping normal force calculation.  
/// **How:** Computes normal forces for a contact with linear overlap compliance and separating relative velocity.  
/// **Why:** Ensures that linear contact mechanics correctly follow Hooke's law combined with normal damping.
/// ====================================================================================================================
#[test]
fn test_normal_linear() {
    let contact = Contact {
        overlap: 0.02,
        eff_stiffness: 1e5,
        eff_damping: 100.0,
        rel_vel: DVec3::new(0.0, 0.0, 0.1), // Separating along normal
        ..Contact::default()
    };

    let (mag, vec) = normal_linear(&contact);
    
    // Expected:
    // f_elastic = 1e5 * 0.02 = 2000.0
    // f_damping = 100.0 * (0.1 * 1.0) = 10.0
    // f_normal_mag = (2000.0 - 10.0) = 1990.0
    assert!((mag - 1990.0).abs() < 1e-6);
    assert_eq!(vec, contact.normal * 1990.0);
}

/// ====================================================================================================================
/// **What:** Validates tangential friction response under pure normal relative motion.  
/// **How:** Evaluates friction damping when tangential relative velocity is zero.  
/// **Why:** Ensures no spurious tangential forces are generated when there is no sliding or shear relative motion.
/// ====================================================================================================================
#[test]
fn test_friction_viscous_damping_zero_tangent() {
    let contact = Contact {
        rel_vel: DVec3::new(0.0, 0.0, -1.0), // Purely normal velocity
        ..Contact::default()
    };
    let f_normal_mag = 100.0;
    let f_t = friction_viscous_damping(f_normal_mag, &contact);
    
    assert_eq!(f_t, DVec3::ZERO);
}

/// ====================================================================================================================
/// **What:** Validates viscous tangential friction when below the Coulomb friction limit.  
/// **How:** Computes tangential force with a moderate relative velocity where ideal damping is smaller than the friction threshold.  
/// **Why:** Ensures linear viscous damping is correctly applied unconstrained when sliding forces remain inside the friction cone.
/// ====================================================================================================================
#[test]
fn test_friction_viscous_damping_within_limit() {
    let contact = Contact {
        rel_vel: DVec3::new(1.0, 0.0, 0.0), // Tangential velocity along X
        eff_damping: 10.0,
        mu: 0.5,
        ..Contact::default()
    };
    let f_normal_mag = 100.0; // Coulomb limit = 0.5 * 100 = 50.0
    
    // v_tang = (1.0, 0.0, 0.0)
    // f_t_ideal = (-10.0, 0.0, 0.0) -> magnitude = 10.0 (<= 50.0 limit)
    let f_t = friction_viscous_damping(f_normal_mag, &contact);
    
    assert!((f_t.x - (-10.0)).abs() < 1e-6);
    assert_eq!(f_t.y, 0.0);
    assert_eq!(f_t.z, 0.0);
}

/// ====================================================================================================================
/// **What:** Validates Coulomb friction limiting and clamping behavior.  
/// **How:** Applies a high tangential velocity where the ideal viscous force exceeds the maximum allowable Coulomb friction threshold.  
/// **Why:** Ensures tangential forces are properly clamped to the friction cone limit to prevent unphysical adhesive or shear locking.
/// ====================================================================================================================
#[test]
fn test_friction_viscous_damping_exceeds_limit() {
    let contact = Contact {
        rel_vel: DVec3::new(10.0, 0.0, 0.0), // High tangential velocity
        eff_damping: 20.0,
        mu: 0.2,
        ..Contact::default()
    };
    let f_normal_mag = 100.0; // Coulomb limit = 0.2 * 100 = 20.0
    
    // f_t_ideal magnitude = 10.0 * 20.0 = 200.0 (exceeds limit of 20.0)
    let f_t = friction_viscous_damping(f_normal_mag, &contact);
    
    // Should clamp to magnitude = 20.0 in the direction opposite to v_tang
    assert!((f_t.length() - 20.0).abs() < 1e-6);
    assert!(f_t.x < 0.0);
}


/// ====================================================================================================================
/// **What:** Checks long-range electrostatic interaction forces between charged particles.  
/// **How:** Assigns opposing unit charges and compares computed forces against analytical Coulomb's Law expectations.  
/// **Why:** Confirms that electric field constants and distance-squared scaling factors are implemented accurately.
/// ====================================================================================================================
#[test]
fn test_coulomb() {
    
    let mut particles = ParticleVec::new();

    let p1 = Particle{
        id : 0,
        position: DVec3::new(1.0,1.0,1.0),
        charge: -1.0,
        ..Particle::default()
    };
    
    let p2 = Particle{
        id: 1,
        position: DVec3::new(2.0,1.0,1.0),
        charge: -1.0,
        ..Particle::default()
    };
    
    particles.push(p1);
    particles.push(p2);
   
    let mut force = DVec3::ZERO;

    let model = CoulombParams::default();

    force = add_coulomb(0, 1, &particles, force, &model);

    const EPS0: f64 = 8.85418782e-12;
    let separation = particles.position[0]-particles.position[1];
    //forces the right are positive
    let coulomb_force = (1.0/(4.0*PI*EPS0))*-1.0*1.0/separation;
    
    assert!(force.x - coulomb_force.x < 1e-6);

}




/// ====================================================================================================================
///
///  neighbours tests
/// 
/// ====================================================================================================================// -----------------------------------------------------------------------------------------------


/// ====================================================================================================================
/// **What:** Validates spatial cell indexing configurations across boundary constraints.  
/// **How:** Builds neighbor matrices under both periodic wrapping and restricted non-periodic conditions.  
/// **Why:** Ensures neighboring box maps are correctly sized and assign sentinel values (`usize::MAX`) appropriately out-of-bounds.
/// ====================================================================================================================
#[test]
fn test_build_neighbour_table() {
    let (mut grid, _settings) = create_grid_and_settings();

    // Periodic
    grid.periodic = [true; 3];
    grid.build_neighbour_table();

    assert_eq!(grid.neighbour_table.len(), 27, "Should be 27 boxes in grid");

    let expected_periodic = vec![
        1, 2, 3, 6, 9, 18, 4, 7, 5, 8, 10, 19, 11, 20, 12, 21, 15, 24, 13, 22, 16, 25, 14, 23, 17, 26,
    ];
    assert_eq!(
        grid.neighbour_table[0], expected_periodic,
        "Neighbours incorrect under periodic boundary conditions"
    );

    // Non-periodic
    grid.periodic = [false; 3];
    grid.neighbour_table = vec![Vec::new(); 27];
    grid.build_neighbour_table();

    assert_eq!(grid.neighbour_table.len(), 27, "Should be 27 boxes in grid");

    let active_neighbours: Vec<usize> = grid.neighbour_table[0]
        .iter()
        .copied()
        .filter(|&x| x != usize::MAX)
        .collect();

    let correct_neighbours = vec![1, 3, 9, 4, 10, 12, 13];
    assert_eq!(
        active_neighbours, correct_neighbours,
        "Should be 7 active neighbour boxes in non-periodic grid for (0,0,0)"
    );
}



/// ====================================================================================================================
/// **What:** Tests mapping from 3D cell coordinates to a flat array index.  
/// **How:** Converts grid coordinate indices `(2, 2, 2)` into a scalar value using `get_1d_idx`.  
/// **Why:** Prevents spatial indexing mismatches by verifying flat buffer layout calculations.
/// ====================================================================================================================
#[test]
fn test_get_1d_idx(){
    let (grid, _settings)=create_grid_and_settings();
    let ix: usize=2;
    let iy: usize=2;
    let iz: usize=2;

    let idx = grid.get_1d_idx(ix,iy,iz);
    assert_eq!(idx, 26, "(2,2,2) should be 26");
}


/// ====================================================================================================================
/// **What:** Checks neighbor offset translation behavior near boundaries.  
/// **How:** Evaluates grid index queries outside bounds in non-periodic mode and across wrapped edges in periodic mode.  
/// **Why:** Guarantees that spatial queries respect domain constraints cleanly without indexing faults.
/// ====================================================================================================================
#[test]
fn test_get_neighbour_1d_idx(){
    let (mut grid, _settings)=create_grid_and_settings();

    let ix: usize=0;
    let iy: usize=0;
    let iz: usize=0;

    //test value outside grid in non-periodic results in None
    grid.periodic = [false;3];
    let new_coords = grid.get_neighbour_1d_idx(ix,iy,iz, [-1,0,0]);
    assert_eq!(new_coords, usize::MAX, "coords should have returned None because outside box");

    //test values in periodic box.
    grid.periodic = [true;3];
    grid.neighbour_table = vec![Vec::new(); 27];

    let new_coords = grid.get_neighbour_1d_idx(ix,iy,iz, [-1,0,0]);
    assert_eq!(new_coords, 2 , "x coord should have wrapped");

}


/// ====================================================================================================================
/// **What:** Tests spatial binning and sorting of particles into cells.  
/// **How:** Passes a particle collection into `bin` and assesses resulting cell offset markers.  
/// **Why:** Ensures particles are properly bucketed and indexed before running neighbor-dependent force passes.
/// ====================================================================================================================
#[test]
fn test_bin() {
    let (mut grid, _settings) = create_grid_and_settings();
    let particles = create_particle_vec();
    grid.bin(&particles);

    assert_eq!(grid.cell_offsets[grid.cell_offsets.len() - 1], particles.position.len());
    assert!(grid.cell_particle_ids.len() == particles.position.len());   
    
}


/// ====================================================================================================================
/// **What:** Validates initial state configuration during the first step of a simulation.  
/// **How:** Initialises grid state with offset reference coordinates and verifies Verlet list population.  
/// **Why:** Ensures everything synchronised correctly prior to regular displacement checks.
/// ====================================================================================================================
#[test]
fn test_first_frame_rebuild() {
    let (mut grid, settings) = create_grid_and_settings();
    let mut particles = create_particle_vec();
    
    particles.position[0] = DVec3::new(1.0,1.0,1.0);
    particles.ref_pos[0] = DVec3::new(5.0,5.0,5.0);

    grid.init(&mut particles);

    assert_eq!(particles.ref_pos[0], particles.position[0]);
    // Verify index 0 and 2 are neighbours (based on create_molecule_vec layout)
    assert!(grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1));
    assert!(!grid.verlet_particle_ids[grid.verlet_offsets[1]..grid.verlet_offsets[2]].contains(&0));
}


/// ====================================================================================================================
/// **What:** Tests particle displacement triggers for Verlet list updates.  
/// **How:** Moves particles incrementally below and past the threshold value ($\text{skin} / 2$).  
/// **Why:** Optimises performance by bypassing expensive re-binning cycles when particle movement is negligible.
/// ====================================================================================================================
#[test]
fn test_skin_displacement_trigger() {
    let (mut grid, settings) = create_grid_and_settings();
    let mut particles = create_molecule_vec();
    
    //pos and ref_pos should be the same
    grid.init(&mut particles);

    // Move 0.09 (less than skin/2 = 0.1), shouldn't rebuild
    particles.position[0] += DVec3::new(0.09, 0.0, 0.0);
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    assert_ne!(particles.ref_pos[0], particles.position[0], "Should not have rebuilt");

    // Move another 0.02 (total 0.11 > skin/2 = 0.2/2)
    particles.position[0] += DVec3::new(0.2, 0.0, 0.0);
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    
    assert_eq!(particles.ref_pos[0], particles.position[0], "Should have triggered rebuild");
}


/// ====================================================================================================================
/// **What:** Confirms that intra-molecular particles are excluded from pairwise neighbor tables.  
/// **How:** Manually attempts to insert a bonded internal pair into the Verlet list structure.  
/// **Why:** Prevents duplicate force calculations and physical conflicts between atoms belonging to the same rigid structure.
/// ====================================================================================================================
#[test]
fn test_molecular_exclusion() {
    let mut particles = ParticleVec::new();

    let com1 = DVec3::new(1.0, 2.0, 3.25);

    let p0 = Particle{
        id: 0,
        molecule_id: 0,
        position : com1 + DVec3::new(1.0, 1.0, 1.0),
        rel_pos : DVec3::new(0.0, 0.0, 0.25),
        omega : DVec3::new(0.0, 1.0, 0.0),
        ..Particle::default()
    };

    let p1 = Particle{
        id: 1,
        molecule_id: 0,
        position : com1 + DVec3::new(1.2, 1.0, 1.0),
        rel_pos : DVec3::new(0.0, 0.0, 0.25),
        omega : DVec3::new(0.0, 1.0, 0.0),
        ..Particle::default()
    };

    particles.push(p0);
    particles.push(p1);
    
    let settings = SimulationSettings {
        skin: 0.2,
        interaction_ptypes: vec![(0, 1, 2.8)],
        ..Default::default()
    };

    let grid = CellGrid::new(particles.length(), &settings);

    // Particles 0 and 1 belong to molecule 0 so shouldn't be in each other's verlet table
    let i = 0;
    let j = 1;

    let pids_b4 = grid.verlet_particle_ids.clone();
    
    // Attempt to add a pair that is physically close but within the same molecule
    grid.add_to_verlet(i, j, &particles);
    
    //println!("aft {:?}", grid.verlet_particle_ids);
    assert_eq!(pids_b4, grid.verlet_particle_ids, "Particle_ids should have stayed the same because particles in same molecule must be excluded");
}


/// ====================================================================================================================
/// **What:** Checks neighbour tracking across periodic domain boundaries.  
/// **How:** Places two interacting entities with distinct molecule IDs near opposite box edges and checks they are logged as neighbours.  
/// **Why:** Ensures boundary-spanning particle interactions are properly captured in Verlet neighborhoods without molecular exclusion interference.
/// ====================================================================================================================
#[test]
fn test_periodic_neighbours() {
    let mut particles = ParticleVec::new();

    let p0 = Particle {
        id: 0,
        ptype: 0,
        molecule_id: 0,
        position: DVec3::new(0.1, 5.0, 5.0),
        ..Particle::default()
    };

    let p1 = Particle {
        id: 1,
        ptype: 0,
        molecule_id: 1,
        position: DVec3::new(8.9, 5.0, 5.0), // Effective wrapped distance is 1.2, within cutoff 2.8
        ..Particle::default()
    };

    particles.push(p0);
    particles.push(p1);

    let settings = SimulationSettings {
        sim_box_size: DVec3::splat(9.0),
        periodic: [true, true, true],
        skin: 0.2,
        interaction_ptypes: vec![(0, 0, 2.8)],
        ..Default::default()
    };

    let mut grid = CellGrid::new(particles.length(), &settings);
    grid.init(&mut particles);
    
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    
    assert!(
        grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1), 
        "Should detect periodic neighbour across box boundaries"
    );
}


/// ====================================================================================================================
/// **What:** Validates ptype-filtered interactions.  
/// **How:** Sets restricted type rules (`interaction_ptypes = vec![(0, 1, 3.0)]`) and checks directional inclusion in the list buffers.  
/// **Why:** Ensures that interactions only occur between the specified particle types and directional pairs.
/// ====================================================================================================================
#[test]
fn test_ptype_interactions() {
    let mut particles = ParticleVec::new();

    let p0 = Particle {
        id: 0,
        ptype: 0,
        molecule_id: 0,
        position: DVec3::new(1.0, 1.0, 1.0),
        ..Particle::default()
    };

    let p1 = Particle {
        id: 1,
        ptype: 1,
        molecule_id: 1,
        position: DVec3::new(2.0, 1.0, 1.0), // Distance is 1.0, well within the 3.0 cutoff
        ..Particle::default()
    };

    particles.push(p0);
    particles.push(p1);

    let settings = SimulationSettings {
        sim_box_size: DVec3::splat(9.0),
        periodic: [true, true, true],
        skin: 0.2,
        interaction_ptypes: vec![(0, 1, 3.0)], // Only type 0 interacting with type 1 is enabled
        ..Default::default()
    };

    let mut grid = CellGrid::new(particles.length(), &settings);
    grid.init(&mut particles);
    grid.check_and_rebuild_neighbours(&mut particles, &settings);

    // Particle 0 (type 0) should have Particle 1 (type 1) in its verlet list
    assert!(
        grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1), 
        "Particle 0 (type 0) should see Particle 1 (type 1)"
    );
    
    // Particle 1 (type 1) should NOT have Particle 0 (type 0) in its list because the (1, 0) interaction rule is not specified
    assert!(
        !grid.verlet_particle_ids[grid.verlet_offsets[1]..grid.verlet_offsets[2]].contains(&0), 
        "Particle 1 (type 1) should not see Particle 0 (type 0)"
    );
}
// --- Test Helpers ---


/// ====================================================================================================================
/// **What:** Validates particle-to-particle collision detection, contact geometry resolution, and non-contact filtering.  
/// **How:** Tests an overlapping particle pair (verifying positive contact properties) and a separated pair (verifying `None`).  
/// **Why:** Ensures that particle interactions correctly compute overlap depth and relative kinematics while ignoring separated bodies.
/// ====================================================================================================================
#[test]
fn test_check_particle_contact() {
    let mut particles = ParticleVec::new();

    let p0 = Particle {
        id: 0,
        ptype: 0,
        position: DVec3::new(1.0, 1.0, 1.0),
        velocity: DVec3::new(1.0, 0.0, 0.0),
        radius: 0.5,
        mass: 1.0,
        ..Particle::default()
    };

    // Overlapping particle (distance = 0.8, combined radius = 1.0 -> overlap = 0.2)
    let p1 = Particle {
        id: 1,
        ptype: 0,
        position: DVec3::new(1.8, 1.0, 1.0),
        velocity: DVec3::new(-1.0, 0.0, 0.0),
        radius: 0.5,
        mass: 1.0,
        ..Particle::default()
    };

    // Separated particle (distance = 2.2, combined radius = 1.0 -> no contact)
    let p2 = Particle {
        id: 2,
        ptype: 0,
        position: DVec3::new(3.2, 1.0, 1.0),
        velocity: DVec3::new(0.0, 0.0, 0.0),
        radius: 0.5,
        mass: 1.0,
        ..Particle::default()
    };

    particles.push(p0);
    particles.push(p1);
    particles.push(p2);

    let settings = SimulationSettings::default();
    let model = CollisionParams::default();

    // 1. Verify contact when overlapping (particles 0 and 1)
    let contact_opt = check_particle_contact(0, 1, &particles, &model, &settings);
    assert!(contact_opt.is_some(), "Particles 0 and 1 should be in contact");
    let contact = contact_opt.unwrap();

    // Check overlap: combined_rad (1.0) - dist (0.8) = 0.2
    assert!((contact.overlap - 0.2).abs() < 1e-6);
    // Normal should point from j to i: (-1.0, 0.0, 0.0) normalized = (-1.0, 0.0, 0.0)
    assert!((contact.normal.x - (-1.0)).abs() < 1e-6);
    assert!(contact.eff_stiffness > 0.0);
    assert!(contact.eff_damping > 0.0);

    // Verify None when separated (particles 0 and 2)
    let no_contact_opt = check_particle_contact(0, 2, &particles, &model, &settings);
    assert!(no_contact_opt.is_none(), "Particles 0 and 2 should not be in contact when outside the combined radius threshold");
}

/// ====================================================================================================================
/// **What:** Validates particle-to-object (surface) collision detection and contact properties.  
/// **How:** Positions a particle penetrating a rectangular boundary surface and evaluates the returned contact specification.  
/// **Why:** Ensures that boundary interactions map cleanly to the unified `Contact` structure for low-level force evaluation.
/// ====================================================================================================================
#[test]
fn test_check_object_contact_rectangle() {
    let mut particles = ParticleVec::new();

    // Particle positioned close to a plane, penetrating slightly (z = 0.4, radius = 0.5 -> overlap = 0.1)
    let p0 = Particle {
        id: 0,
        ptype: 0,
        position: DVec3::new(0.0, 0.0, 0.4),
        velocity: DVec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
        mass: 1.0,
        ..Particle::default()
    };

    // Particle positioned safely outside the boundary plane (z = 0.6, radius = 0.5 -> separated)
    let p1 = Particle {
        id: 1,
        ptype: 0,
        position: DVec3::new(0.0, 0.0, 0.6),
        velocity: DVec3::new(0.0, 0.0, 0.0),
        radius: 0.5,
        mass: 1.0,
        ..Particle::default()
    };

    particles.push(p0);
    particles.push(p1);

    // Create a horizontal rectangular plane centered at origin lying on the xy-plane
    let corners = [
        DVec3::new(-2.0,  2.0, 0.0), // Top-Left
        DVec3::new( 2.0,  2.0, 0.0), // Top-Right
        DVec3::new( 2.0, -2.0, 0.0), // Bottom-Right
        DVec3::new(-2.0, -2.0, 0.0), // Bottom-Left
    ];
    let rect = RectSpec::new(corners, Srgba::new(1.0, 1.0, 1.0, 1.0), true);
    let object = ObjectSpec::Rectangle(rect);

    let settings = SimulationSettings::default();
    let model = CollisionParams::default();

    // 1. Verify contact when penetrating
    let contact_opt = check_object_contact(0, &object, &particles, &model, &settings);
    assert!(contact_opt.is_some(), "Particle 0 should be in contact with the rectangle");
    let contact = contact_opt.unwrap();

    // Check overlap: radius (0.5) - distance to closest point (0.4) = 0.1
    assert!((contact.overlap - 0.1).abs() < 1e-6);
    // Normal should point along +z (from plane to particle)
    assert!((contact.normal.z - 1.0).abs() < 1e-6);
    assert!(contact.eff_stiffness > 0.0);

    // 2. Verify None when moved out of contact
    let no_contact_opt = check_object_contact(1, &object, &particles, &model, &settings);
    assert!(no_contact_opt.is_none(), "Particle 1 should not be in contact when outside the radius threshold");
}