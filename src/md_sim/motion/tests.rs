

use glam::{DVec3, DMat3};

use crate::md_sim::utils::{create_particle_vec, create_single_molecule, create_molecule_vec, setup_single_molecule_data};
use crate::md_sim::particle::{calculate_molecule_com, calculate_kinetic_energy, calculate_total_angular_momentum};
use crate::md_sim::motion::{enforce_boundary,integrate_rigid_bodies, integrate_rigid_bodies_correct, integrate_singleparticle_correct, integrate_singleparticle_update, change_rad};
use crate::md_sim::SimulationSettings;

//-------------------------------------------------------------------------------------------------------
// Testing integration functions
//-------------------------------------------------------------------------------------------------------

/// **What:** Tests the single-particle prediction step (`integrate_singleparticle_update`).  
/// **How:** Applies a known force to a point particle over a fixed time step ($\Delta t$).  
/// **Why:** Verifies that velocities receive the correct half-step acceleration and positions advance correctly using Velocity Verlet.
#[test]
fn test_integrate_singleparticle_update() {
    let mut particles = create_particle_vec(); // Particles at (1,2,3)
    let mut settings = SimulationSettings::default();
    settings.dt = 0.1;
    settings.sim_box_size = DVec3::new(10.0, 10.0, 10.0);

    // Force of 10.0 on Particle 0 (mass is 1.0) -> Accel = 10.0
    let mut forces = vec![DVec3::ZERO; particles.len()];
    forces[0] = DVec3::new(10.0, 0.0, 0.0);
    let mut torques = vec![DVec3::ZERO; particles.len()];
    torques[0] = DVec3::new(10.0, 0.0, 0.0);

    // Initial state: pos=1.0, vel=1.0
    // Expected Vel Half-step: 1.0 + (10.0 * 0.05) = 1.5
    // Expected Pos Full-step: 1.0 + (1.5 * 0.1) = 1.15
    integrate_singleparticle_update(&forces, &mut particles, &settings);

    assert!((particles.velocity[0].x - 1.5).abs() < 1e-6);
    assert!((particles.position[0].x - 1.15).abs() < 1e-6);
}

/// **What:** Tests the single-particle correction step (`integrate_singleparticle_correct`).  
/// **How:** Feeds pre-predicted intermediate velocities and new forces into the correction function.  
/// **Why:** Ensures the final half-step velocity update correctly completes the Velocity Verlet cycle.
#[test]
fn test_integrate_singleparticle_correct() {
    let mut particles = create_particle_vec();
    let mut settings = SimulationSettings::default();
    settings.dt = 0.1;

    // Force of 10.0 in X direction. mass is 1.0, so Accel = 10.0
    let forces = vec![DVec3::new(10.0, 0.0, 0.0); particles.len()];

    // Manually set a "pre-predicted" state.
    // Let's assume the particle started at vel 1.0.
    // After the first half-kick (update), vel should be: 1.0 + (10.0 * 0.05) = 1.5
    for vel in &mut particles.velocity {
        *vel = DVec3::new(1.5, 0.0, 0.0);
    }

    // Perform the Correction (The second half-kick)
    // Mathematically: v_final = v_half + (a_new * half_dt)
    // v_final = 1.5 + (10.0 * 0.05) = 2.0
    integrate_singleparticle_correct(&forces, &mut particles, &settings);

    // Verify
    for vel in &particles.velocity {
        assert!((vel.x - 2.0).abs() < 1e-6, "Velocity correction failed to reach 2.0");
    }
}

//---------------------------------------------------------------------
// Rigid Body integrations
//---------------------------------------------------------------------



/// **What:** Tests rigid-body momentum conservation under zero forces.  
/// **How:** Evolves a multi-particle rigid molecule over a timestep with zero applied forces or torques.  
/// **Why:** Verifies that Center of Mass (COM) velocity and angular momentum ($\omega$) remain perfectly constant in free flight.
#[test]
fn test_integrate_rigid_body_conservation() {
    let settings = SimulationSettings { dt: 0.1, ..Default::default() };
    let mut particles = create_molecule_vec();// Uses a molecule which consists of 2 spheres of mass 0.5 and 1.5 separated by 1.0.
    let mol_data = setup_single_molecule_data(&particles); 
    
    // Calculate Initial State
    let (_mass,_com,initial_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);       
    let initial_omega = particles.omega[0];

    // Perform one step with zero forces
    integrate_rigid_bodies(&vec![DVec3::ZERO; 2], &vec![DVec3::ZERO; 2], &mut particles, &mol_data, &settings);
    integrate_rigid_bodies_correct(&vec![DVec3::ZERO; 2], &vec![DVec3::ZERO; 2], &mut particles, &mol_data, &settings);

    // Verify Conservation
    let (_,_,final_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);

    assert!((final_com_vel - initial_com_vel).length() < 1e-12, "COM Velocity changed!");
    assert!((particles.omega[0] - initial_omega).length() < 1e-12, "Omega changed!");
}

/// **What:** Tests rigid-body translation under a uniform external field (gravity).  
/// **How:** Applies a directional gravitational force vector across a rigid molecule's constituent particles.  
/// **Why:** Verifies that external forces correctly alter the Center of Mass vertical velocity without causing artificial lateral drift
#[test]
fn test_integrate_rigid_body_gravity() {
    let dt = 0.1;
    let settings = SimulationSettings { dt, ..Default::default() };

    let mut particles = create_molecule_vec();
    let mol_data = setup_single_molecule_data(&particles);
    
    // Gravity acting only on Z
    let gravity = DVec3::new(0.0, 0.0, -9.81);
    let mut forces = vec![DVec3::ZERO; particles.len()];

    for i in 0..particles.len() {
        forces[i] = gravity * particles.mass[i];
    }
    
    let torques = vec![DVec3::ZERO; 2];

    // Integration
    integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
    integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);
    let (_,_,final_com_vel) = calculate_molecule_com(&vec![0, 1], &particles);

    // Verify: Only Z-velocity should be affected by gravity
    let expected_z_vel = 1.0 + (gravity.z * dt);

    assert!((final_com_vel[2] - expected_z_vel).abs() < 1e-12, "Z-axis gravity integration failed!");
    assert!((final_com_vel[0] - 0.75).abs() < 1e-12, "X-axis velocity should remain unchanged!");
}


/// **What:** Tests molecule stability when zero net torque is applied.  
/// **How:** Passes zero force and torque to an initially moving, spinning molecule
/// **Why:** Verifies that angular velocity ($\omega$) remains unaffected and COM translation is conserved.
#[test]
fn test_molecule_rotation_no_torque() {
    let dt = 0.1;
    let settings = SimulationSettings { dt, ..Default::default() };
    let mut particles = create_single_molecule();
    let mol_data = setup_single_molecule_data(&particles);
    let molecule = mol_data.get(&0).expect("0 should exist");

    let initial_omega = particles.omega[0];
    let (_, _, initial_com_vel) = calculate_molecule_com(&molecule.pids, &particles);

    let forces = vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(0.0, 0.0, 0.0)];
    let torques = vec![DVec3::ZERO, DVec3::ZERO];

    integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
    integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);

    let final_omega = particles.omega[0];
    let (_, _, final_com_vel) = calculate_molecule_com(&molecule.pids, &particles);

    assert!((final_omega.y - initial_omega.y).abs() < 1e-12, "Angular velocity change failed!");    
    assert!((final_com_vel.x - initial_com_vel.x).abs() < 1e-12, "COM X-velocity should be conserved!");       
}


/// **What:** Tests rigid-body rotational dynamics under a physical torque couple.  
/// **How:** Applies equal and opposite forces to individual particles in a molecule to induce rotational acceleration.  
/// **Why:** Validates that external forces produce torque. Checks that translational velocity stays the same but rotation speeds up.
#[test]
fn test_molecule_rotation_torque() {
    let dt = 0.1;
    let settings = SimulationSettings { dt, ..Default::default() };
    let mut particles = create_single_molecule();
    let mol_data = setup_single_molecule_data(&particles);
    let molecule = mol_data.get(&0).expect("0 should exist");

    let (_, _, init_com_vel) = calculate_molecule_com(&molecule.pids, &particles);

    // Apply a force couple: P0 pushed in +X, P1 pushed in -X
    // This creates rotation around the Y-axis.
    let forces = vec![DVec3::new(1.0, 0.0, 0.0), DVec3::new(-1.0, 0.0, 0.0)];
    let torques = vec![DVec3::ZERO, DVec3::ZERO];

    // Integration
    integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
    let mid_omega = particles.omega[0];
    assert!((mid_omega.y - 0.08695652173913045).abs() < 1e-12, "Angular velocity change failed!"); 

    integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);   
    let final_omega = particles.omega[0];
    let (_, _, final_com_vel) = calculate_molecule_com(&molecule.pids, &particles);
    
    assert!((final_omega.y - 0.17390975591781438).abs() < 1e-12, "Angular velocity change failed!"); 
    assert!((final_com_vel.x - init_com_vel.x).abs() < 1e-12, "COM X-velocity should be conserved!");       
}

//-------------------------------------------------------------------------------------------------------
// special functions
//-------------------------------------------------------------------------------------------------------

/// **What:** Tests boundary condition enforcement (`enforce_boundary`).  
/// **How:** Tests coordinates exceeding box bounds under both periodic flags (`true`) and elastic reflection flags (`false`).  
/// **Why:** Ensures correct modular wrapping for periodic cells and precise wall position/velocity inversion for solid boundaries.
#[test]
fn test_enforce_boundary() {
    let sim_box = DVec3::new(10.0, 10.0, 10.0);

    // 1. Test Periodic Wrapping
    let mut pos = DVec3::new(12.0, -2.0, 5.0);
    let mut vel = DVec3::new(1.0, 1.0, 1.0);
    enforce_boundary(&mut pos, &mut vel, sim_box, [true, true, true]);
    
    assert!((pos.x - 2.0).abs() < 1e-9);
    assert!((pos.y - 8.0).abs() < 1e-9);
    assert!((pos.z - 5.0).abs() < 1e-9);

    // 2. Test Elastic Lower Bound Reflection
    let mut pos = DVec3::new(-1.0, 5.0, 5.0);
    let mut vel = DVec3::new(-1.0, 0.0, 0.0);
    enforce_boundary(&mut pos, &mut vel, sim_box, [false, false, false]);
    
    assert!((pos.x - 1.0).abs() < 1e-9); 
    assert!((vel.x - 1.0).abs() < 1e-9); 

    // 3. Test Elastic Upper Bound Reflection
    let mut pos = DVec3::new(11.0, 5.0, 5.0);
    let mut vel = DVec3::new(1.0, 0.0, 0.0);
    enforce_boundary(&mut pos, &mut vel, sim_box, [false, false, false]);
    
    assert!((pos.x - 9.0).abs() < 1e-9); 
    assert!((vel.x - -1.0).abs() < 1e-9); 
}

/// **What:** Tests type-specific particle swelling/growth (`change_rad`).  
/// **How:** Targets a specific particle type ID for incremental radius scaling while leaving other types untouched.  
/// **Why:** Verifies selective growth control for compression protocols without polluting unrelated particle categories.
#[test]
fn test_particle_growth_by_type() {
    let mut particles = create_particle_vec();
    let original_rad_0 = particles.radius[0];
    let original_rad_1 = particles.radius[1];

    change_rad(&mut particles, 1);

    assert_eq!(particles.radius[0], original_rad_0, "Ptype 0 should not have grown");
    assert!(particles.radius[1] > original_rad_1, "Ptype 1 should have grown");
    
    let expected = original_rad_1 * 1.00001;
    assert!((particles.radius[1] - expected).abs() < 1e-9);
}

/// **What:** Tests long-term numerical stability and energy drift over thousands of steps.  
/// **How:** Runs a rigid molecule through 10,000 unforced simulation steps using small timesteps.  
/// **Why:** Checks for systematic energy or angular momentum accumulation bugs in the Velocity Verlet and orientation update loop.
#[test]
fn test_numerical_stability() {
    let dt = 0.0001; 
    let num_steps = 10_000;
    let settings = SimulationSettings { dt, ..Default::default() };
    
    let mut particles = create_single_molecule();
    let mol_data = setup_single_molecule_data(&particles);
    
    let initial_energy = calculate_kinetic_energy(&particles, &mol_data);
    let initial_angular_momentum = calculate_total_angular_momentum(&particles, &mol_data);
    
    for _ in 0..num_steps {
        let forces = vec![DVec3::ZERO; particles.len()];
        let torques = vec![DVec3::ZERO; particles.len()];
        
        integrate_rigid_bodies(&forces, &torques, &mut particles, &mol_data, &settings);
        integrate_rigid_bodies_correct(&forces, &torques, &mut particles, &mol_data, &settings);
    }
    
    let final_energy = calculate_kinetic_energy(&particles, &mol_data);
    let final_angular_momentum = calculate_total_angular_momentum(&particles, &mol_data);
    
    let energy_drift = (final_energy - initial_energy).abs() / initial_energy;
    let momentum_drift = (final_angular_momentum - initial_angular_momentum).length();
    
    assert!(energy_drift < 1e-10, "Energy drift too high: {}", energy_drift);
    assert!(momentum_drift < 1e-10, "Angular momentum not conserved! Drift: {}", momentum_drift);
}