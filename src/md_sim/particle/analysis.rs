
use glam::{DVec3, DMat3};
use std::collections::HashMap;

use super::ParticleVec;
use super::{MoleculeData, calculate_molecule_com};

/// Calculates total system kinetic energy (Translational + Rotational)
pub fn calculate_kinetic_energy(particles: &ParticleVec, molecules: &HashMap<usize, MoleculeData>) -> f64 {
    let mut total_ke = 0.0;

    // Translational KE: sum(0.5 * m_total * v_com^2)
    for mol in molecules.values() {
        let (m_total, _, v_com) = calculate_molecule_com(&mol.pids, particles);
        total_ke += 0.5 * m_total * v_com.length_squared();
    }

    // Rotational KE: sum(0.5 * omega * I_global * omega)
    for mol in molecules.values() {
        let omega = particles.omega[mol.pids[0]]; // Assuming uniform omega for rigid body
        let orientation = particles.orientation[mol.pids[0]];
        
        let rot_mat = DMat3::from_quat(orientation);
        let i_global = rot_mat * mol.inertia * rot_mat.transpose();
        
        total_ke += 0.5 * omega.dot(i_global * omega);
    }

    total_ke
}

/// Calculates total angular momentum (Orbital + Spin)
pub fn calculate_total_angular_momentum(particles: &ParticleVec, molecules: &HashMap<usize, MoleculeData>) -> DVec3 {
    let mut total_l = DVec3::ZERO;

    for mol in molecules.values() {
        let (total_mass, com_pos, v_com) = calculate_molecule_com(&mol.pids, particles);
        
        // Spin Angular Momentum (S = I_global * omega)
        let omega = particles.omega[mol.pids[0]];
        let rot_mat = DMat3::from_quat(particles.orientation[mol.pids[0]]);
        let i_global = rot_mat * mol.inertia * rot_mat.transpose();
        
        let spin_l = i_global * omega;
        
        // Orbital Angular Momentum relative to COM
        // Sum_i ( (r_i - R_com) x m_i * (v_i - V_com) )
        let mut orbital_l = DVec3::ZERO;
        for &idx in &mol.pids {
            let r_rel = particles.position[idx] - com_pos;
            let v_rel = particles.velocity[idx] - v_com;
            orbital_l += r_rel.cross(particles.mass[idx] * v_rel);
        }

        total_l += spin_l + orbital_l;
    }

    total_l
}
