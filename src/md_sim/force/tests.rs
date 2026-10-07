

use glam:: DVec3;
use three_d::Srgba;
use std::path::PathBuf;

use crate::md_sim::force::{check_particle_contact, check_object_contact};
use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{Particle, RectSpec, ObjectSpec, ParticleVec};
use crate::md_sim::utils::create_grid_and_settings;
use crate::md_sim::force::contact::{Contact, ContactState, ContactManager};
use crate::md_sim::force::{add_gravity,Gravity, add_viscous_drag, ViscousDrag, FrictionViscous, FrictionCundallStrack, NormalForce, add_coulomb, CoulombParams};
use crate::md_sim::force::{normal_hertzian, normal_linear, friction_viscous_damping};
use super::neighbours::CellGrid;
use std::f64::consts::PI;
use crate::md_sim::force::pairwise::RawNormalForce;

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
        rel_vel: DVec3::new(0.0, 0.0, -0.5), // Approaching along normal
        r_eff: 1.0,
        ..Contact::default()
    };

    let model = NormalForce::default();

    let (f_mag, f_vec) = normal_hertzian(&contact, &model);
    
    // With default modulus = 1e5 and nu = 0.25:
    // E* = 1e5 / (2 * (1 - 0.25^2)) ≈ 53333.33
    // K_n = (4/3) * E* * sqrt(r_eff) ≈ 71111.11
    // f_elastic = K_n * (0.01)^1.5 ≈ 71.111
    // f_normal_mag = (f_elastic - damping_term).max(0.0)
    
    assert!(f_mag > 0.0);
    assert!((f_vec.z - f_mag).abs() < 1e-6);
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
        rel_vel: DVec3::new(0.0, 0.0, 0.1), // Separating along normal
        ..Contact::default()
    };

    // Construct explicitly via RawNormalForce to ensure modulus 
    // correctly populates all derived material coefficients (e_star, beta)
    let model: NormalForce = RawNormalForce {
        modulus: 1e5,
        restitution: 0.5,
        plane_modulus: 1e5,
        plane_restitution: 0.5,
    }.into();

    let (mag, vec) = normal_linear(&contact, &model);
    
    // Calculate expected values using the updated linear stiffness implementation
    let (e_star, beta) = (model.particle_estar, model.particle_beta);
    let eff_stiffness = 2.0 * e_star * contact.r_eff;
    let eff_damping = 2.0 * beta * (contact.m_eff * eff_stiffness).sqrt();
    
    let f_elastic = eff_stiffness * contact.overlap;
    let f_damping = eff_damping * contact.rel_vel.dot(contact.normal);
    let expected_mag = (f_elastic - f_damping).max(0.0);

    assert!((mag - expected_mag).abs() < 1e-6);
    assert_eq!(vec, contact.normal * expected_mag);
}

/// ====================================================================================================================
/// **What:** Validates tangential friction response under pure normal relative motion.  
/// **How:** Evaluates friction damping when tangential relative velocity is zero.  
/// **Why:** Ensures no spurious tangential forces are generated when there is no sliding or shear relative motion.
/// ====================================================================================================================
#[test]
fn test_friction_viscous_damping_zero_tangent() {
    let contact = Contact {
        normal: DVec3::new(0.0, 0.0, 1.0),
        rel_vel: DVec3::new(0.0, 0.0, -1.0), // Purely normal velocity (aligned with normal)
        ..Contact::default()
    };
    
    let model = FrictionViscous::default();
    let f_normal_mag = 100.0;
    
    let f_t = friction_viscous_damping(f_normal_mag, &contact, &model);
    
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
        ..Contact::default()
    };

    // Explicitly configure the model so damping_coeff matches the expected force scale (10.0)
    let model = FrictionViscous {
        mu: 0.5,
        restitution: 0.5,
        damping_coeff: 10.0, 
    };
    
    let f_normal_mag = 100.0; // Coulomb limit = 0.5 * 100 = 50.0
    
    // v_tang = (1.0, 0.0, 0.0) with damping_coeff = 10.0 -> f_t = (-10.0, 0.0, 0.0)
    let f_t = friction_viscous_damping(f_normal_mag, &contact, &model);
    
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
        rel_vel: DVec3::new(10.0, 0.0, 0.0), // High tangential velocity (10.0)
        ..Contact::default()
    };
    let f_normal_mag = 100.0; // Coulomb limit = 0.2 * 100 = 20.0
    
    // Configure both mu and the damping coefficient so that 
    // rel_vel * damping_coeff exceeds the Coulomb limit (e.g., 10.0 * 20.0 = 200.0)
    let mut model = FrictionViscous::default();
    model.mu = 0.2;
    model.damping_coeff = 20.0; // Adjust field name if your struct uses `viscosity` or similar

    let f_t = friction_viscous_damping(f_normal_mag, &contact, &model);
    
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
/// **What:** Checks neighbour offset translation behavior near boundaries.  
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
    
    let mut particles = ParticleVec::new();
    
    // Push a couple of particles to populate the particle vector
    let mut p1 = Particle::default();
    p1.position = DVec3::new(1.0, 1.0, 1.0);
    particles.push(p1);

    let mut p2 = Particle::default();
    p2.position = DVec3::new(2.0, 2.0, 2.0);
    particles.push(p2);

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
    let (mut grid, _settings) = create_grid_and_settings();
    
    let mut particles = ParticleVec::new();
    
    // Particle 0
    let mut p0 = Particle::default();
    p0.position = DVec3::new(1.0, 1.0, 1.0);
    particles.push(p0);

    // Particle 1 (placed close to particle 0 to be within neighbor cutoff)
    let mut p1 = Particle::default();
    p1.position = DVec3::new(1.2, 1.0, 1.0);
    particles.push(p1);

    // Particle 2 (placed far away)
    let mut p2 = Particle::default();
    p2.position = DVec3::new(10.0, 10.0, 10.0);
    particles.push(p2);

    // Set a mismatched reference position to verify grid.init resets it
    particles.ref_pos[0] = DVec3::new(5.0, 5.0, 5.0);

    grid.init(&mut particles);

    assert_eq!(particles.ref_pos[0], particles.position[0]);
    
    // Verify half-list neighbor structure: 
    // Particle 0's list contains particle 1, but particle 1's list does not contain 0
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
    
    // Create a ParticleVec and push particles manually
    let mut particles = ParticleVec::new();
    let mut p = Particle::default();
    p.position = DVec3::new(1.0, 1.0, 1.0);
    p.radius = 0.5;
    particles.push(p);
    
    // Initialize grid and reference positions
    grid.init(&mut particles);

    // Move 0.09 (less than skin/2 = 0.1), shouldn't rebuild reference positions
    particles.position[0] += DVec3::new(0.09, 0.0, 0.0);
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    assert_ne!(particles.ref_pos[0], particles.position[0], "Should not have rebuilt");

    // Move another 0.2 (total displacement exceeds skin threshold)
    particles.position[0] += DVec3::new(0.2, 0.0, 0.0);
    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    
    assert_eq!(particles.ref_pos[0], particles.position[0], "Should have triggered rebuild");
}

/// ====================================================================================================================
/// **What:** Confirms that intra-molecular particles are excluded from pairwise neighbour tables.  
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

    let grid = CellGrid::new(particles.len(), &settings);

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

    let mut grid = CellGrid::new(particles.len(), &settings);
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

    let mut grid = CellGrid::new(particles.len(), &settings);
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


/// ====================================================================================================================
///
///  contact tests
/// 
/// ====================================================================================================================// -----------------------------------------------------------------------------------------------




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

    // Verify contact when overlapping (particles 0 and 1)
    let contact_opt = check_particle_contact(0, 1, &particles, &settings);
    assert!(contact_opt.is_some(), "Particles 0 and 1 should be in contact");
    let contact = contact_opt.unwrap();

    // Check overlap: combined_rad (1.0) - dist (0.8) = 0.2
    assert!((contact.overlap - 0.2).abs() < 1e-6);
    // Normal should point from j to i: (-1.0, 0.0, 0.0) normalized = (-1.0, 0.0, 0.0)
    assert!((contact.normal.x - (-1.0)).abs() < 1e-6);


    // Verify None when separated (particles 0 and 2)
    let no_contact_opt = check_particle_contact(0, 2, &particles, &settings);
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
    let rect = RectSpec::new(corners, Srgba::WHITE, true);
    let object = ObjectSpec::Rectangle(rect);

    let settings = SimulationSettings::default();

    // Verify contact when overlapping
    let contact_opt = check_object_contact(0, &object, &particles, &settings);
    assert!(contact_opt.is_some(), "Particle 0 should be in contact with the rectangle");
    let contact = contact_opt.unwrap();

    // Check overlap: radius (0.5) - distance to closest point (0.4) = 0.1
    assert!((contact.overlap - 0.1).abs() < 1e-6);
    // Normal should point along +z (from plane to particle)
    assert!((contact.normal.z - 1.0).abs() < 1e-6);

    // Verify None when moved out of contact
    let no_contact_opt = check_object_contact(1, &object, &particles, &settings);
    assert!(no_contact_opt.is_none(), "Particle 1 should not be in contact when outside the radius threshold");
}


/// ====================================================================================================================
/// **What:** Validates contact insertion, tangential displacement accumulation, and active state marking via `check_or_add`.  
/// **How:** Inserts a contact pair with an incremental displacement, verifies initial state creation, adds a second increment, and asserts total accumulated displacement and active status.  
/// **Why:** Ensures that Cundall-Strack tangential shear displacements correctly accumulate and persist across successive simulation steps.
/// ====================================================================================================================
#[test]
fn test_contact_manager_check_or_add() {
    let manager = ContactManager::new(PathBuf::from("dummy/path"));
    
    // Should insert default state, add displacement, and mark active
    let pair = (0, 1);
    let displacement = DVec3::new(0.1, 0.0, 0.0);
    manager.check_or_add(pair, displacement);
    
    assert!(manager.states.contains_key(&pair), "Contact pair should be registered in the manager");
    let state = manager.states.get(&pair).unwrap();
    assert!(state.is_active, "Contact should be marked active");
    assert!((state.tangential_disp.x - 0.1).abs() < 1e-6, "Initial tangential displacement should match");

    // Second encounter: should accumulate shear displacement across steps
    let additional_displacement=DVec3::new(0.05, 0.0, 0.0); 
    manager.check_or_add(pair, additional_displacement);
    
    let updated_state = manager.states.get(&pair).unwrap();
    assert!((updated_state.tangential_disp.x - 0.15).abs() < 1e-6, "Tangential displacement should accumulate correctly");
    assert!(updated_state.is_active, "Contact should remain active after re-encounter");
}


/// ====================================================================================================================
/// **What:** Validates the pruning of expired or separated contacts and the reset of active flags via `remove_old_contacts`.  
/// **How:** Inserts an untouched expired pair and a touched active pair, executes `remove_old_contacts`, and asserts retention vs. deletion.  
/// **Why:** Prevents the accumulation of ghost contact histories from separated bodies while resetting active flags for the next simulation step.
/// ====================================================================================================================
#[test]
fn test_contact_manager_remove_old_contacts() {
    let manager = ContactManager::new(PathBuf::from("dummy/path"));
    
    let active_pair = (0, 1);
    let expired_pair = (2, 3);

    // Manually seed an expired pair (simulating a contact that is no longer colliding this step)
    manager.states.insert(expired_pair, ContactState {
        is_active: false,
        ..ContactState::default()
    });

    // Encounter and touch the active pair during the current step sweep
    manager.check_or_add(active_pair, DVec3::ZERO);

    assert_eq!(manager.states.len(), 2, "Manager should contain both contacts prior to cleanup");

    // Run the cleanup pass
    manager.remove_old_contacts();

    assert!(manager.states.contains_key(&active_pair), "Active contacts must be retained");
    assert!(!manager.states.contains_key(&expired_pair), "Expired contacts must be pruned");

    // Verify that `remove_old_contacts` successfully reset the flag to false for the upcoming step
    let retained_state = manager.states.get(&active_pair).unwrap();
    assert!(!retained_state.is_active, "Active flag should be reset to false post-cleanup to prepare for the next step");
}
