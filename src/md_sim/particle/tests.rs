
use glam::{DVec3, DQuat, DMat3};
use three_d::Srgba;

use crate::md_sim::utils::{create_molecule_vec, setup_single_molecule_data};

use super::*;
const NULL_ID: usize = usize::MAX;

//-------------------------------------------------------------------------------
// Tests analysis.rs
// -----------------------------------------------------------------------------

#[test]
fn test_rigidbody_ke(){
    let p = create_molecule_vec();
    let molecules = setup_single_molecule_data(&p);

    //p consists of m[1.5, 0.5], rel_pos[0.25, -0.75], vel[(1,1,1), (0,1,1)]
    let total_mass = p.mass[0] + p.mass[1];
    let com = (p.mass[0]*p.position[0] + p.mass[1]*p.position[1])/total_mass;
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
    assert!(false);
}

#[test]
fn test_total_ang_momentum(){
    assert!(false);
}
//-------------------------------------------------------------------------------
// Tests geometry.rs
// -----------------------------------------------------------------------------


#[test]
fn test_calculate_com(){
    assert!(false);
}

#[test]
fn test_calc_inertia(){
    assert!(false);
}
//-------------------------------------------------------------------------------
// Tests particle.rs
// -----------------------------------------------------------------------------

#[test]
fn test_particle_new() {

    let id = 1;
    let position = DVec3::new(1.0, 2.0, 3.0);
    let velocity = DVec3::new(0.1, 0.2, 0.3);
    let orientation= DQuat::IDENTITY;
    let omega= DVec3::ZERO;
    let color = Srgba::new(255, 0, 0, 255);
    let radius: f64 = 0.5;
    let density: f64=1.0;
    
    let ptype = 1;
    
    let mass = (4.0 / 3.0) * std::f64::consts::PI * radius.powf(3f64) * density;
    let particle = Particle::new(id, NULL_ID, ptype, position,DVec3::ZERO, velocity, orientation, omega, radius, density, 0.0, color);

    assert_eq!(particle.id, id);
    assert_eq!(particle.position, position);
    assert_eq!(particle.velocity, velocity);
    assert_eq!(particle.color, color);
    assert_eq!(particle.radius, radius);
    assert_eq!(particle.mass, mass);
}

