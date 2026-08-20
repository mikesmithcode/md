use glam::{DVec3,DMat3};

use super::ParticleVec;
/// Stores structural and inertial properties for a rigid multi-particle molecule.
#[derive(Debug)]
pub struct MoleculeData {
    /// Indices of the constituent particles belonging to this molecule.
    pub pids: Vec<usize>,
    /// Pre-calculated body-frame inertia tensor of the rigid assembly.
    pub inertia: DMat3, 
}

impl MoleculeData {
    /// Creates a new `MoleculeData` instance and computes its permanent body-frame inertia tensor.
    ///
    /// # Arguments
    ///
    /// * `pids` - Vector of particle indices comprising the molecule.
    /// * `particles` - Reference to the shared particle state buffers.
    ///
    /// # Returns
    ///
    /// * `Self` - An initialized molecule specification.
    pub fn new(pids: Vec<usize>, particles: &ParticleVec) -> Self {           
        // Calculate constant body-frame inertia tensor
        let inertia = calculate_molecule_inertia(&pids, particles);
        
        Self { pids, inertia }
    }
}

/// Calculates the total mass, center of mass position, and linear velocity of a molecule.
///
/// # Arguments
///
/// * `pids` - Slice of particle indices defining the molecule.
/// * `particles` - Reference to the particle state buffers containing positions, velocities, and masses.
///
/// # Returns
///
/// * `(f64, DVec3, DVec3)` - A tuple containing total mass, center of mass position, and center of mass velocity.
pub fn calculate_molecule_com(pids: &[usize], particles: &ParticleVec) -> (f64, DVec3, DVec3) {
    let mut total_mass = 0.0;
    let mut com_pos = DVec3::ZERO;
    let mut vel = DVec3::ZERO;

    for &idx in pids {
        let mass = particles.mass[idx];
        total_mass += mass;
        com_pos += particles.position[idx] * mass;
        vel += particles.velocity[idx] * mass;
    }
    com_pos /= total_mass;
    vel /= total_mass;

    (total_mass, com_pos, vel)
}

/// Calculates the rigid-body inertia tensor for a multi-particle molecule using static body-frame positions.
///
/// # Arguments
///
/// * `pids` - Slice of particle indices comprising the molecule.
/// * `particles` - Reference to the particle state buffers containing radii, masses, and static relative positions.
///
/// # Returns
///
/// * `DMat3` - The integrated 3x3 inertia tensor matrix for the rigid assembly.
pub fn calculate_molecule_inertia(pids: &[usize], particles: &ParticleVec) -> DMat3 {
    let mut total_inertia = DMat3::ZERO;
    // Note: Do not pass COM here; use the relative positions directly.
    for &idx in pids {
        let m = particles.mass[idx];
        let r = particles.rel_pos[idx]; // The static body-frame vector
        
        let i_val = 0.4 * m * particles.radius[idx].powi(2);
        let i_local = DMat3::from_diagonal(DVec3::splat(i_val));

        let r2 = r.dot(r);
        let outer_prod = DMat3::from_cols(
            DVec3::new(r.x * r.x, r.x * r.y, r.x * r.z),
            DVec3::new(r.y * r.x, r.y * r.y, r.y * r.z),
            DVec3::new(r.z * r.x, r.z * r.y, r.z * r.z),
        );
        
        total_inertia += i_local + (DMat3::IDENTITY * r2 - outer_prod) * m;
    }
    total_inertia
}