

use glam::{DQuat, DVec2, DVec3};
use three_d::Srgba;

use crate::md_sim::SimulationSettings;
use crate::md_sim::particle::{Particle, RectSpec, TriSpec, SimulationModel, FrictionParams, ParticleVec, SurfaceKinematics};
use super::objects::particle_contact_response;
use crate::md_sim::utils::{create_particle_vec,create_molecule_vec, create_grid_and_settings, assert_dvec3_near};
use crate::md_sim::utils::InteractionContext;

use super::{add_weight,add_viscous_drag, add_particle_particle_collision, add_coulomb};
use super::neighbours::CellGrid;
use std::f64::consts::PI;

// -----------------------------------------------------------------
// Test single particle forces
// -----------------------------------------------------------------


/// **What:** Verifies that gravitational body forces are correctly calculated and applied.  
/// **How:** Applies weight to the first particle using `add_weight` and inspects the resulting force vector.  
/// **Why:** Ensures that gravitational acceleration maps cleanly to the vertical force buffer for single-body dynamics.
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

/// **What:** Validates velocity-dependent Stokes' law viscous drag calculations.  
/// **How:** Computes drag force on a particle with known velocity and radius against a specified fluid viscosity.  
/// **Why:** Ensures that drag damping forces scale correctly relative to particle dimensions and surrounding medium parameters.
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


// -----------------------------------------------------------------
// Test pair particle forces
// -----------------------------------------------------------------

/// **What:** Tests viscoelastic contact mechanics and energy dissipation during particle collisions.  
/// **How:** Evaluates interaction forces under relative compression (moving together) versus restitution (moving apart).  
/// **Why:** Confirms that damping increases total force magnitude strictly during compression to correctly model collision energy loss.  
#[test]
fn test_particle_particle_collision() {
    let particles = create_particle_vec();
    
    // Bundle params into the specific Enum variant
    let model = SimulationModel::Frictional(FrictionParams {
        stiffness: 1000.0,
        damping: 50.0,
        ..Default::default()
    });

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        periodic: [true;3],
        parallel: true,
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

    (force, _) = add_particle_particle_collision(0, 1, &particles, force, DVec3::ZERO, &settings);

    assert!(force.x < 0.0, "Force should be repulsive for particle 0");
    let force_with_damping = force.length();

    // --- Case B: Restitution (Moving away) ---
    force = DVec3::ZERO; // Reset force buffer
    particles.velocity[0] = DVec3::new(-1.0, 0.0, 0.0);
    particles.velocity[1] = DVec3::new(1.0, 0.0, 0.0);

    (force, _ )=add_particle_particle_collision(0, 1, &particles, force,DVec3::ZERO, &settings);
    let force_no_damping = force.length();

    // force_with_damping (Compression) should be > force_no_damping (Restitution).
    assert!(force_with_damping > force_no_damping, "Damping must increase total force magnitude during compression");
}


/// **What:** Checks long-range electrostatic interaction forces between charged particles.  
/// **How:** Assigns opposing unit charges and compares computed forces against analytical Coulomb's Law expectations.  
/// **Why:** Confirms that electric field constants and distance-squared scaling factors are implemented accurately.
#[test]
fn test_coulomb() {
    let mut particles = create_molecule_vec();
    
    particles.charge[0] = 1.0;
    particles.charge[1] = -1.0;

    // Bundle params into the specific Enum variant
    let model = SimulationModel::Frictional(FrictionParams {
        stiffness: 1000.0,
        damping: 50.0,
        ..Default::default()
    });

    // Initialise the full SimulationSettings struct
    let settings = SimulationSettings {
        dt: 0.001,             
        sim_box_size: DVec3::new(10.0, 10.0, 10.0),
        periodic: [true;3],
        parallel: true,
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
// neighbours tests
// -----------------------------------------------------------------------------------------------

/// **What:** Validates spatial cell indexing configurations across boundary constraints.  
/// **How:** Builds neighbor matrices under both periodic wrapping and restricted non-periodic conditions.  
/// **Why:** Ensures neighboring box maps are correctly sized and assign sentinel values (`usize::MAX`) appropriately out-of-bounds.
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



/// **What:** Tests mapping from 3D cell coordinates to a flat array index.  
/// **How:** Converts grid coordinate indices `(2, 2, 2)` into a scalar value using `get_1d_idx`.  
/// **Why:** Prevents spatial indexing mismatches by verifying flat buffer layout calculations.
#[test]
fn test_get_1d_idx(){
    let (grid, _settings)=create_grid_and_settings();
    let ix: usize=2;
    let iy: usize=2;
    let iz: usize=2;

    let idx = grid.get_1d_idx(ix,iy,iz);
    assert_eq!(idx, 26, "(2,2,2) should be 26");
}


/// **What:** Checks neighbor offset translation behavior near boundaries.  
/// **How:** Evaluates grid index queries outside bounds in non-periodic mode and across wrapped edges in periodic mode.  
/// **Why:** Guarantees that spatial queries respect domain constraints cleanly without indexing faults.
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

/// **What:** Tests spatial binning and sorting of particles into cells.  
/// **How:** Passes a particle collection into `bin` and assesses resulting cell offset markers.  
/// **Why:** Ensures particles are properly bucketed and indexed before running neighbor-dependent force passes.
#[test]
fn test_bin() {
    let (mut grid, _settings) = create_grid_and_settings();
    let particles = create_particle_vec();
    grid.bin(&particles);

    assert_eq!(grid.cell_offsets[grid.cell_offsets.len() - 1], particles.position.len());
    assert!(grid.cell_particle_ids.len() == particles.position.len());   
    
}


/// **What:** Validates initial state configuration during the first step of a simulation.  
/// **How:** Initialises grid state with offset reference coordinates and verifies Verlet list population.  
/// **Why:** Ensures everything synchronised correctly prior to regular displacement checks.
#[test]
fn test_first_frame_rebuild() {
    let (mut grid, settings) = create_grid_and_settings();
    let mut particles = create_particle_vec();
    
    particles.position[0] = DVec3::new(1.0,1.0,1.0);
    particles.ref_pos[0] = DVec3::new(5.0,5.0,5.0);

    grid.init(&mut particles, &settings);

    assert_eq!(particles.ref_pos[0], particles.position[0]);
    // Verify index 0 and 2 are neighbours (based on create_molecule_vec layout)
    assert!(grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1));
    assert!(!grid.verlet_particle_ids[grid.verlet_offsets[1]..grid.verlet_offsets[2]].contains(&0));
}

/// **What:** Tests particle displacement triggers for Verlet list updates.  
/// **How:** Moves particles incrementally below and past the threshold value ($\text{skin} / 2$).  
/// **Why:** Optimises performance by bypassing expensive re-binning cycles when particle movement is negligible.
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

/// **What:** Confirms that intra-molecular particles are excluded from pairwise neighbor tables.  
/// **How:** Manually attempts to insert a bonded internal pair into the Verlet list structure.  
/// **Why:** Prevents duplicate force calculations and physical conflicts between atoms belonging to the same rigid structure.
#[test]
fn test_molecular_exclusion() {
    let (grid, settings) = create_grid_and_settings();
    let particles = create_molecule_vec();
    
    // Particles 0 and 1 belong to molecule 0 so shouldn't be in each other's verlet table
    let i = 0;
    let j = 1;
    
    let ctx = InteractionContext{
        sim_box_size: settings.sim_box_size,
        periodic: settings.periodic,
        interaction_ptypes: &settings.interaction_ptypes,
        search_radius_sq: (settings.cutoff + settings.skin).powi(2),
    };

    let pids_b4 = grid.verlet_particle_ids.clone();
    //println!("b4 {:?}", grid.verlet_particle_ids);
    // Attempt to add a pair that is physically close but within the same molecule
    CellGrid::add_to_verlet(i, j, &particles, &ctx);
    
    //println!("aft {:?}", grid.verlet_particle_ids);
    assert_eq!(pids_b4, grid.verlet_particle_ids, "Particle_ids should have stayed the same because particles in same molecule must be excluded");
}

/// **What:** Checks neighbour tracking across periodic domain boundaries.  
/// **How:** Places two interacting entities near opposite box edges and checks they are logged as neighbours.
/// **Why:** Ensures boundary-spanning particle interactions are properly captured in Verlet neighborhoods.
#[test]
fn test_periodic_neighbours() {
    let (mut grid, settings) = create_grid_and_settings();
    let mut particles = create_particle_vec();
    
    // Place particles across periodic boundary
    particles.position[0] = DVec3::new(0.1, 5.0, 5.0);
    particles.position[1] = DVec3::new(8.9, 5.0, 5.0); // 1.2 distance, within cutoff 3.0
    

    grid.check_and_rebuild_neighbours(&mut particles, &settings);
    
    assert!(grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1), "Should detect periodic neighbour");
}

/// **What:** Validates ptype-filtered interactions.  
/// **How:** Sets restricted type rules (`interaction_ptype = vec![[0, 1]]`) and checks directional inclusion in the list buffers.  
/// **Why:** Ensures that interactions only occur between the types of particles and in the direction specified.
#[test]
fn test_ptype_interactions() {
    let (mut grid, settings) = create_grid_and_settings();
    let mut particles = create_particle_vec();
    grid.init(&mut particles, &settings);

    // Ball (id=0) should have ball (id=1) in its list because interaction_ptype = vec![[0,1]]
    assert!(grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&1), "0 should see 1");
    
    // Ball (id=1) should NOT have Ball (id=0) in its list because interaction_ptype not specified.
    assert!(!grid.verlet_particle_ids[grid.verlet_offsets[0]..grid.verlet_offsets[1]].contains(&0), "1 should not see 0");
}

// --- Test Helpers ---

fn test_color() -> Srgba {
    Srgba::new(255, 0, 0, 255)
}

fn setup_test_settings(stiffness: f64, damping: f64, mu: f64) -> SimulationSettings {
    SimulationSettings {
        model: SimulationModel::Frictional(FrictionParams {
            plane_stiffness: stiffness,
            plane_damping: damping,
            plane_mu: mu,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn setup_single_particle(pos: DVec3, vel: DVec3, omega: DVec3, radius: f64) -> ParticleVec {
    let mut particles = ParticleVec::new();
    let particle = Particle::new(
        0,                        // id
        0,                        // molecule_id
        0,                        // ptype
        pos,                      // position
        DVec3::ZERO,              // rel_pos
        vel,                      // velocity
        DQuat::IDENTITY,          // orientation
        omega,                    // omega
        radius,                   // radius
        1000.0,                   // density
        0.0,                      // charge
        Srgba::new(255, 255, 255, 255), // colour
        true,                     // visible
    );
    
    // Auto-generated by soa_derive to push all fields simultaneously
    particles.push(particle); 
    particles
}



// --- Mock Implementation ---

#[derive(Default)]
struct MockSurface {
    closest_pt: DVec3,
    surface_velocity: DVec3,
}

impl SurfaceKinematics for MockSurface {
    fn closest_point(&self, _particle_pos: DVec3) -> DVec3 {
        self.closest_pt
    }

    fn velocity_at_point(&self, _point: DVec3) -> DVec3 {
        self.surface_velocity
    }
}

// --- Particle-Contact Response Tests ---

#[test]
fn test_no_collision_out_of_range() {
    let settings = setup_test_settings(1000.0, 10.0, 0.5);
    let surface = MockSurface::default();

    let particles = setup_single_particle(DVec3::new(0.0, 0.0, 2.0), DVec3::ZERO, DVec3::ZERO, 1.0);

    let (force, torque) = particle_contact_response(
        0,
        &particles,
        &surface,
        DVec3::ZERO,
        DVec3::ZERO,
        &settings,
    );

    assert_eq!(force, DVec3::ZERO);
    assert_eq!(torque, DVec3::ZERO);
}

#[test]
fn test_pure_normal_spring_force() {
    let settings = setup_test_settings(1000.0, 0.0, 0.0);
    let surface = MockSurface::default();

    let particles = setup_single_particle(DVec3::new(0.0, 0.0, 0.8), DVec3::ZERO, DVec3::ZERO, 1.0);

    let (force, torque) = particle_contact_response(
        0,
        &particles,
        &surface,
        DVec3::ZERO,
        DVec3::ZERO,
        &settings,
    );

    let expected_force = DVec3::new(0.0, 0.0, 200.0);
    assert_dvec3_near(force, expected_force, 1e-12);
    assert_eq!(torque, DVec3::ZERO);
}

#[test]
fn test_normal_damping_force() {
    let settings = setup_test_settings(1000.0, 50.0, 0.0);
    let surface = MockSurface::default();

    let particles = setup_single_particle(
        DVec3::new(0.0, 0.0, 0.8),
        DVec3::new(0.0, 0.0, -2.0),
        DVec3::ZERO,
        1.0,
    );

    let (force, _) = particle_contact_response(
        0,
        &particles,
        &surface,
        DVec3::ZERO,
        DVec3::ZERO,
        &settings,
    );

    let expected_force = DVec3::new(0.0, 0.0, 300.0);
    assert_dvec3_near(force, expected_force, 1e-12);
}

#[test]
fn test_friction_and_torque() {
    let settings = setup_test_settings(1000.0, 100.0, 0.3);
    let surface = MockSurface::default();

    let particles = setup_single_particle(
        DVec3::new(0.0, 0.0, 0.9),
        DVec3::new(10.0, 0.0, 0.0),
        DVec3::ZERO,
        1.0,
    );

    let (force, torque) = particle_contact_response(
        0,
        &particles,
        &surface,
        DVec3::ZERO,
        DVec3::ZERO,
        &settings,
    );

    let expected_force = DVec3::new(-30.0, 0.0, 100.0);
    assert_dvec3_near(force, expected_force, 1e-12);

    let expected_torque = DVec3::new(0.0, 27.0, 0.0);
    assert_dvec3_near(torque, expected_torque, 1e-12);
}

// --- RectSpec Kinematics Tests ---

#[test]
fn test_closest_point_on_rectspec() {
    let rect = RectSpec {
        id: 1,
        centre: DVec3::ZERO,
        velocity: DVec3::ZERO,
        orientation: DQuat::IDENTITY,
        omega: DVec3::ZERO,
        half_size: DVec2::new(2.0, 1.0),
        vertices: [DVec3::ZERO; 4],
        colour: test_color(),
        visible: true,
    };

    let eps = 1e-12;

    // Corners
    assert_dvec3_near(rect.closest_point(DVec3::new(3.0, 2.0, 0.5)), DVec3::new(2.0, 1.0, 0.0), eps);
    assert_dvec3_near(rect.closest_point(DVec3::new(-3.0, -2.0, -0.5)), DVec3::new(-2.0, -1.0, 0.0), eps);

    // Edges
    assert_dvec3_near(rect.closest_point(DVec3::new(4.0, 0.0, 0.5)), DVec3::new(2.0, 0.0, 0.0), eps);
    assert_dvec3_near(rect.closest_point(DVec3::new(0.0, 3.0, -0.5)), DVec3::new(0.0, 1.0, 0.0), eps);

    // Faces
    assert_dvec3_near(rect.closest_point(DVec3::new(0.5, 0.5, 3.0)), DVec3::new(0.5, 0.5, 0.0), eps);
    assert_dvec3_near(rect.closest_point(DVec3::new(-0.2, -0.3, -2.5)), DVec3::new(-0.2, -0.3, 0.0), eps);
}

#[test]
fn test_rectspec_transformed_closest_point() {
    let rot_y90 = DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2);
    let centre = DVec3::new(5.0, 5.0, 5.0);

    let rect = RectSpec {
        id: 2,
        centre,
        velocity: DVec3::ZERO,
        orientation: rot_y90,
        omega: DVec3::ZERO,
        half_size: DVec2::new(2.0, 1.0),
        vertices: [DVec3::ZERO; 4],
        colour: test_color(),
        visible: true,
    };

    let rect_normal = rect.normal();
    let p_above = rect.centre + rect_normal * 2.0;
    
    assert_dvec3_near(rect.closest_point(p_above), rect.centre, 1e-12);
}

// --- TriSpec Kinematics Tests ---

#[test]
fn test_closest_point_on_trispec() {
    let v0 = DVec3::new(0.0, 0.0, 0.0);
    let v1 = DVec3::new(2.0, 0.0, 0.0);
    let v2 = DVec3::new(0.0, 2.0, 0.0);

    let tri = TriSpec::new([v0, v1, v2], test_color(), true);
    let eps = 1e-12;

    // Vertices
    assert_dvec3_near(tri.closest_point(DVec3::new(-1.0, -1.0, 0.5)), tri.vertices[0], eps);
    assert_dvec3_near(tri.closest_point(DVec3::new(3.0, 0.0, -0.5)), tri.vertices[1], eps);
    assert_dvec3_near(tri.closest_point(DVec3::new(0.0, 3.0, 0.5)), tri.vertices[2], eps);

    // Edges
    assert_dvec3_near(tri.closest_point(DVec3::new(1.0, -2.0, 0.5)), DVec3::new(1.0, 0.0, 0.0), eps);
    assert_dvec3_near(tri.closest_point(DVec3::new(-2.0, 1.0, -0.5)), DVec3::new(0.0, 1.0, 0.0), eps);
    assert_dvec3_near(tri.closest_point(DVec3::new(2.0, 2.0, 1.0)), DVec3::new(1.0, 1.0, 0.0), eps);

    // Faces
    assert_dvec3_near(tri.closest_point(DVec3::new(0.5, 0.5, 3.0)), DVec3::new(0.5, 0.5, 0.0), eps);
    assert_dvec3_near(tri.closest_point(DVec3::new(0.2, 0.3, -2.5)), DVec3::new(0.2, 0.3, 0.0), eps);
}

#[test]
fn test_trispec_transformed_closest_point() {
    let v0 = DVec3::new(0.0, 0.0, 0.0);
    let v1 = DVec3::new(2.0, 0.0, 0.0);
    let v2 = DVec3::new(0.0, 2.0, 0.0);

    let mut tri = TriSpec::new([v0, v1, v2], test_color(), true);

    let rot_y90 = DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2);
    tri.transform(DVec3::new(5.0, 5.0, 5.0), Some(rot_y90));

    let p_above = tri.centre + tri.normal() * 2.0;

    assert_dvec3_near(tri.closest_point(p_above), tri.centre, 1e-12);
}






