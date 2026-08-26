
use glam::{DVec2, DVec3, DQuat, DMat3};
use three_d::Srgba;

use crate::md_sim::utils::{create_molecule_vec, setup_single_molecule_data, assert_dvec3_near};

use super::*;
const NULL_ID: usize = usize::MAX;

//-------------------------------------------------------------------------------
// Tests analysis.rs
// -----------------------------------------------------------------------------

/// **What:** Validates the total kinetic energy calculation for a rigid molecule.  
/// **How:** Computes expected translational and rotational kinetic energy components manually from state properties and compares against `calculate_kinetic_energy`.  
/// **Why:** Ensures that rigid-body mechanics correctly combine center-of-mass translation and global inertia-tensor rotation.
#[test]
fn test_rigidbody_ke(){
    let p = create_molecule_vec();
    let molecules = setup_single_molecule_data(&p);

    //p consists of m[1.5, 0.5], rel_pos[0.25, -0.75], vel[(1,1,1), (0,1,1)]
    let total_mass = p.mass[0] + p.mass[1];
    //let com = (p.mass[0]*p.position[0] + p.mass[1]*p.position[1])/total_mass;
    let v_com = (p.mass[0]*p.velocity[0] + p.mass[1]*p.velocity[1])/total_mass;
    let ke_t = 0.5*total_mass*v_com.length_squared();

    let rot_mat = DMat3::from_quat(p.orientation[0]);
    let mol = molecules.get(&0).unwrap();
    let i_global = rot_mat * mol.inertia * rot_mat.transpose();
    let ke_rot = 0.5 * p.omega[0].dot(i_global * p.omega[0]);

    let expected_ke = ke_t + ke_rot;
    println!("expect ke {:?}", expected_ke);


    let ke = calculate_kinetic_energy(&p, &molecules);
    println!("ke {:?}", ke);
    // Expected total = 4.0 + 0.5 = 4.5
    
    assert!((ke - expected_ke).abs() < 1e-10, 
            "Expected total KE of 4.5 (4.0 trans + 0.5 rot), but got {}", ke);
}


/// **What:** Tests conservation and calculation of total angular momentum for multi-particle rigid bodies.  
/// **How:** Accumulates spin and orbital angular momentum components across constituent molecule particles.  
/// **Why:** Confirms that rotational dynamics correctly account for off-center offsets relative to the center of mass.
#[test]
fn test_total_ang_momentum() {
    let p = create_molecule_vec();
    
    // Set up molecule data specifically for the isolated vector
    let molecules = setup_single_molecule_data(&p);

    let ang_mom = calculate_total_angular_momentum(&p, &molecules);

    let expected = DVec3::new(0.0, 0.95, 0.0);
    assert_dvec3_near(ang_mom, expected, 1e-12);
}

//-------------------------------------------------------------------------------
// Tests geometry.rs
// -----------------------------------------------------------------------------

/// **What:** Tests center-of-mass and velocity calculation routines for composite structures.  
/// **How:** Passes a slice of particle indices into `calculate_molecule_com` and verifies mass-weighted averages.  
/// **Why:** Ensures kinematic reference frames are centered correctly for rigid-body updates.  
#[test]
fn test_calculate_com() {
    let p = create_molecule_vec();
    let pids = vec![0, 1];

    let (total_mass, com_pos, com_vel) = calculate_molecule_com(&pids, &p);

    assert!((total_mass - 2.0).abs() < 1e-12);
    assert_dvec3_near(com_pos, DVec3::new(1.0, 2.0, 3.5), 1e-12);
    assert_dvec3_near(com_vel, DVec3::new(0.75, 1.0, 1.0), 1e-12);
}

/// **What:** Verifies analytical inertia tensor generation against known geometric configurations.  
/// **How:** Evaluates `calculate_molecule_inertia` for a structured particle pair and checks diagonal and off-diagonal tensor terms.  
/// **Why:** Prevents rotational inertia anomalies during rigid-body torque applications.  
#[test]
fn test_calc_inertia() {
    let p = create_molecule_vec();
    let pids = vec![0, 1];

    let inertia = calculate_molecule_inertia(&pids, &p);

    let expected_inertia = DMat3::from_cols_array(&[
        0.575, 0.0,   0.0,
        0.0,   0.575, 0.0,
        0.0,   0.0,   0.20,
    ]);

    for col in 0..3 {
        assert_dvec3_near(inertia.col(col), expected_inertia.col(col), 1e-12);
    }
}
//-------------------------------------------------------------------------------
// Tests particle.rs
// -----------------------------------------------------------------------------

/// **What:** Validates property assignment and mass derivation for newly initialized particles.  
/// **How:** Calls `Particle::new` with explicit dimensions and density parameters, comparing outputs against expected volume-mass calculations.  
/// **Why:** Confirms that physical attributes like mass and radius initialize consistently across simulation entities.
#[test]
fn test_particle_new() {

    let id = 1;
    let position = DVec3::new(1.0, 2.0, 3.0);
    let velocity = DVec3::new(0.1, 0.2, 0.3);
    let orientation= DQuat::IDENTITY;
    let omega= DVec3::ZERO;
    let colour = Srgba::new(255, 0, 0, 255);
    let radius: f64 = 0.5;
    let density: f64=1.0;
    
    let ptype = 1;
    
    let mass = (4.0 / 3.0) * std::f64::consts::PI * radius.powf(3f64) * density;
    let particle = Particle::new(id, NULL_ID, ptype, position,DVec3::ZERO, velocity, orientation, omega, radius, density, 0.0, colour, true);

    assert_eq!(particle.id, id);
    assert_eq!(particle.position, position);
    assert_eq!(particle.velocity, velocity);
    assert_eq!(particle.colour, colour);
    assert_eq!(particle.radius, radius);
    assert_eq!(particle.mass, mass);
}

//-------------------------------------------------------------------
// Test Objects
//---------------------------------------------------------



/// **What:** Validates initialization and corner vertex generation for a `RectSpec` plane.  
/// **How:** Instantiates a rectangle with explicit corner coordinates and verifies calculated center, half-sizes, and reconstructed vertices.  
/// **Why:** Ensures that spatial layout and local-to-world basis frame transformations build correctly from input vertices.  
#[test]
fn test_rect_new_and_vertices() {
    let vertices = [
        DVec3::new(-1.0, 1.0, 0.0), // Top-Left
        DVec3::new( 1.0, 1.0, 0.0), // Top-Right
        DVec3::new( 1.0,-1.0, 0.0), // Bottom-Right
        DVec3::new(-1.0,-1.0, 0.0), // Bottom-Left
    ];
    let rect = RectSpec::new(vertices, Srgba::WHITE, true);

    assert_eq!(rect.centre, DVec3::ZERO);
    assert_eq!(rect.half_size, DVec2::new(1.0, 1.0));
    for (orig, calc) in vertices.iter().zip(rect.vertices.iter()) {
        assert!((*orig - *calc).length() < 1e-10);
    }
}

/// **What:** Tests rigid-body translation and rotational updates on a rectangular plane.  
/// **How:** Applies a combined position displacement and quaternion rotation delta to a `RectSpec` and inspects updated coordinates.  
/// **Why:** Confirms that movement and orientation updates correctly recompute world-space vertex arrays.  
#[test]
fn test_rect_transform() {
    let vertices = [
        DVec3::new(-1.0, 1.0, 0.0),
        DVec3::new( 1.0, 1.0, 0.0),
        DVec3::new( 1.0,-1.0, 0.0),
        DVec3::new(-1.0,-1.0, 0.0),
    ];
    let mut rect = RectSpec::new(vertices, Srgba::WHITE, true);

    let translation = DVec3::new(5.0, 0.0, 0.0);
    let rotation = Some(DQuat::from_axis_angle(DVec3::Z, std::f64::consts::FRAC_PI_2));
    
    rect.transform(translation, rotation);

    assert_eq!(rect.centre, DVec3::new(5.0, 0.0, 0.0));
    // After 90-degree Z rotation, top-left (-1, 1, 0) should rotate to (-1, -1, 0) + offset (5, 0, 0) = (4, -1, 0)
    assert!((rect.vertices[0] - DVec3::new(4.0, -1.0, 0.0)).length() < 1e-10);
}

/// **What:** Validates time-integration step behavior for linear velocities.  
/// **How:** Advances a rectangle using `step` with non-zero linear velocities over a fixed time step `dt`.  
/// **Why:** Ensures that dynamic positional and rotational progression update correctly over time increments.  
#[test]
fn test_rect_step() {
    let vertices = [
        DVec3::new(-1.0, 1.0, 0.0),
        DVec3::new( 1.0, 1.0, 0.0),
        DVec3::new( 1.0,-1.0, 0.0),
        DVec3::new(-1.0,-1.0, 0.0),
    ];
    let mut rect = RectSpec::new(vertices, Srgba::WHITE, true);

    let vel = DVec3::new(2.0, 0.0, 0.0);
    let omega = DVec3::ZERO;
    let dt = 0.5;

    rect.step(vel, omega, dt);

    // updates stored velocity
    assert_eq!(rect.velocity, vel);
    // Moves centre from (0,0,0) to (1,0,0)
    assert_eq!(rect.centre, DVec3::new(1.0, 0.0, 0.0));
}

/// **What:** Validates time-integration step behavior for angular velocities (pure rotation).  
/// **How:** Advances a rectangle using `step` with zero linear velocity and a non-zero angular velocity vector over a fixed time step `dt`.  
/// **Why:** Ensures that rotational progression, orientation quaternions, and vertex reorientations update correctly around the center of mass without translational drift.  
#[test]
fn test_rect_step_angular() {
    let vertices = [
        DVec3::new(-1.0, 1.0, 0.0),
        DVec3::new( 1.0, 1.0, 0.0),
        DVec3::new( 1.0,-1.0, 0.0),
        DVec3::new(-1.0,-1.0, 0.0),
    ];
    let mut rect = RectSpec::new(vertices, Srgba::WHITE, true);

    let vel = DVec3::ZERO;
    // Rotate around the Z axis by pi/2 radians per second
    let omega = DVec3::new(0.0, 0.0, std::f64::consts::FRAC_PI_2);
    let dt = 1.0;

    rect.step(vel, omega, dt);

    // Updates stored velocities
    assert_eq!(rect.velocity, vel);
    assert_eq!(rect.omega, omega);
    // Center should remain unchanged for pure rotation
    assert_eq!(rect.centre, DVec3::ZERO);
    // After 90-degree Z rotation, top-left (-1, 1, 0) should rotate to (-1, -1, 0)
    assert!((rect.vertices[0] - DVec3::new(-1.0, -1.0, 0.0)).length() < 1e-10);
}

/// **What:** Checks that invalid geometry triggers a validation panic.  
/// **How:** Constructs a degenerate rectangle with overlapping or collinear vertices and invokes validation.  
/// **Why:** Prevents malformed or zero-area geometric structures from entering simulation state pipelines.  
#[test]
#[should_panic(expected = "RectSpec error")]
fn test_rect_validate_panic() {
    let degenerate_vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 0.0),
    ];
    let _rect = RectSpec::new(degenerate_vertices, Srgba::WHITE, true);
}

/// **What:** Validates initialization and corner vertex generation for a `TriSpec` triangle.  
/// **How:** Instantiates a triangle with explicit corner coordinates and verifies calculated center, half-sizes/geometry, and reconstructed vertices.  
/// **Why:** Ensures that spatial layout and local-to-world basis frame transformations build correctly from input vertices.  
#[test]
fn test_trispec_new_and_vertices() {
    let vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let tri = TriSpec::new(vertices, Srgba::WHITE, true);

    // Center is the average of the 3 vertices: (1/3, 1/3, 0)
    assert!((tri.centre - DVec3::new(1.0 / 3.0, 1.0 / 3.0, 0.0)).length() < 1e-10);
    for (orig, calc) in vertices.iter().zip(tri.vertices.iter()) {
        assert!((*orig - *calc).length() < 1e-10);
    }
}

/// **What:** Tests rigid-body translation and rotational updates on a triangular plane.  
/// **How:** Applies a combined position displacement and quaternion rotation delta to a `TriSpec` and inspects updated coordinates.  
/// **Why:** Confirms that movement and orientation updates correctly recompute world-space vertex arrays.  
#[test]
fn test_trispec_transform() {
    let vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let mut tri = TriSpec::new(vertices, Srgba::WHITE, true);

    let translation = DVec3::new(5.0, 0.0, 0.0);
    let rotation = Some(DQuat::from_axis_angle(DVec3::Z, std::f64::consts::FRAC_PI_2));
    
    tri.transform(translation, rotation);

    assert!((tri.centre - DVec3::new(5.0 + 1.0/3.0, 1.0/3.0, 0.0)).length() < 1e-10);
    tri.validate();
}

/// **What:** Validates time-integration step behavior for linear velocities.  
/// **How:** Advances a triangle using `step` with non-zero linear velocities over a fixed time step `dt`.  
/// **Why:** Ensures that dynamic positional and rotational progression update correctly over time increments.  
#[test]
fn test_trispec_step_linear() {
    let vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let mut tri = TriSpec::new(vertices, Srgba::WHITE, true);

    let vel = DVec3::new(2.0, 0.0, 0.0);
    let omega = DVec3::ZERO;
    let dt = 0.5;

    tri.step(vel, omega, dt);

    // Updates stored velocity
    assert_eq!(tri.velocity, vel);
    // Moves centre from (1/3, 1/3, 0) to (1 + 1/3, 1/3, 0)
    assert!((tri.centre - DVec3::new(1.0 + 1.0/3.0, 1.0/3.0, 0.0)).length() < 1e-10);
}

/// **What:** Validates time-integration step behavior for angular velocities (pure rotation).  
/// **How:** Advances a triangle using `step` with zero linear velocity and a non-zero angular velocity vector over a fixed time step `dt`.  
/// **Why:** Ensures that rotational progression, orientation quaternions, and vertex reorientations update correctly around the center of mass without translational drift.  
#[test]
fn test_trispec_step_angular() {
    let vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let mut tri = TriSpec::new(vertices, Srgba::WHITE, true);

    let vel = DVec3::ZERO;
    // Rotate around the Z axis by pi/2 radians per second
    let omega = DVec3::new(0.0, 0.0, std::f64::consts::FRAC_PI_2);
    let dt = 1.0;

    tri.step(vel, omega, dt);

    // Updates stored velocities
    assert_eq!(tri.velocity, vel);
    assert_eq!(tri.omega, omega);
    // Center should remain unchanged for pure rotation
    assert!((tri.centre - DVec3::new(1.0/3.0, 1.0/3.0, 0.0)).length() < 1e-10);
}

/// **What:** Checks that invalid geometry triggers a validation panic.  
/// **How:** Constructs a degenerate triangle with collinear or overlapping vertices and invokes validation.  
/// **Why:** Prevents malformed or zero-area geometric structures from entering simulation state pipelines.  
#[test]
#[should_panic(expected = "is degenerate")]
fn test_trispec_validate_panic() {
    let degenerate_vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, 0.0), // Collinear points yield zero area
    ];
    let _tri = TriSpec::new(degenerate_vertices, Srgba::WHITE, true);
}

// --- Helpers ---
fn dummy_colour() -> Srgba {
    Srgba::new(255, 255, 255, 255)
}

fn sample_rect() -> RectSpec {
    // Construct a 2x2 square in the XY plane centred at origin (0, 0, 0)
    let vertices = [
        DVec3::new(-1.0, 1.0, 0.0),  // Top-Left
        DVec3::new(1.0, 1.0, 0.0),   // Top-Right
        DVec3::new(1.0, -1.0, 0.0),  // Bottom-Right
        DVec3::new(-1.0, -1.0, 0.0), // Bottom-Left
    ];
    RectSpec::new(vertices, dummy_colour(), true)
}

fn sample_tri() -> TriSpec {
    // Right-angled triangle in the XY plane
    let vertices = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, 0.0),
        DVec3::new(0.0, 2.0, 0.0),
    ];
    TriSpec::new(vertices, dummy_colour(), true)
}

// --- RectSpec SurfaceKinematics Tests ---

#[test]
fn test_rect_closest_point_interior() {
    let rect = sample_rect();
    // Point directly above the center
    let p = DVec3::new(0.0, 0.0, 5.0);
    let closest = rect.closest_point(p);
    assert!((closest - DVec3::ZERO).length() < 1e-12);
}

#[test]
fn test_rect_closest_point_clamped() {
    let rect = sample_rect();
    // Point far outside top-right corner (half_size is 1.0, 1.0)
    let p = DVec3::new(5.0, 5.0, 2.0);
    let closest = rect.closest_point(p);
    assert!((closest - DVec3::new(1.0, 1.0, 0.0)).length() < 1e-12);
}

#[test]
fn test_rect_velocity_at_point() {
    let mut rect = sample_rect();
    rect.velocity = DVec3::new(1.0, 0.0, 0.0);
    rect.omega = DVec3::new(0.0, 0.0, 2.0); // Rotating around Z-axis

    // Point offset from centre along X-axis
    let pt = rect.centre + DVec3::new(1.0, 0.0, 0.0);
    let v = rect.velocity_at_point(pt);

    // v = (1, 0, 0) + (0, 0, 2) x (1, 0, 0) = (1, 2, 0)
    let expected = DVec3::new(1.0, 2.0, 0.0);
    assert!((v - expected).length() < 1e-12);
}

// --- TriSpec SurfaceKinematics Tests ---

#[test]
fn test_tri_closest_point_inside() {
    let tri = sample_tri();
    // Point hovering above interior (0.5, 0.5, 0.0)
    let p = DVec3::new(0.5, 0.5, 3.0);
    let closest = tri.closest_point(p);
    assert!((closest - DVec3::new(0.5, 0.5, 0.0)).length() < 1e-12);
}

#[test]
fn test_tri_closest_point_vertex() {
    let tri = sample_tri();
    // Point nearest to vertex A (0, 0, 0)
    let p = DVec3::new(-2.0, -2.0, 0.0);
    let closest = tri.closest_point(p);
    assert!((closest - DVec3::ZERO).length() < 1e-12);
}

#[test]
fn test_tri_closest_point_edge() {
    let tri = sample_tri();
    // Point outside the hypotenuse
    let p = DVec3::new(2.0, 2.0, 0.0);
    let closest = tri.closest_point(p);
    assert!((closest - DVec3::new(1.0, 1.0, 0.0)).length() < 1e-12);
}

#[test]
fn test_tri_velocity_at_point() {
    let mut tri = sample_tri();
    tri.velocity = DVec3::new(0.0, -1.0, 0.0);
    tri.omega = DVec3::new(1.0, 0.0, 0.0); // Pitching around X-axis

    let pt = tri.centre + DVec3::new(0.0, 2.0, 0.0);
    let v = tri.velocity_at_point(pt);

    // v = (0, -1, 0) + (1, 0, 0) x (0, 2, 0) = (0, -1, 2)
    let expected = DVec3::new(0.0, -1.0, 2.0);
    assert!((v - expected).length() < 1e-12);
}
