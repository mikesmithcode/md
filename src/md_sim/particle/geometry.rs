use glam::{DVec3,DMat3};

use super::{ParticleVec, Particle};
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
#[test]
fn test_calc_inertia() {
    let mut particles = ParticleVec::new();
    
    // Manually push or resize your ParticleVec and set SoA fields directly:
    // (Assuming standard SoA push or index assignment methods)
    
    let mut particle = Particle::default();
    particle.id=0;
    particle.mass = 0.5;
    particle.radius = 0.5;
    particle.rel_pos = DVec3::new(-0.75, 0.0, 0.0); // Relative to COM (-0.75 from center of mass)
    particles.push(particle); // or however your ParticleVec adds elements

    let mut particle = Particle::default();
    particle.id=1;
    particle.mass = 1.5;
    particle.radius = 0.5;
    particle.rel_pos = DVec3::new(0.25, 0.0, 0.0); // Relative to COM (+0.25 from center of mass)
    particles.push(particle); // or however your ParticleVec adds elements
    
    
    let pids = vec![0,1];
    let inertia = calculate_molecule_inertia(&pids, &particles);

    let expected_inertia = DMat3::from_cols_array(&[
        0.20,  0.0,   0.0,
        0.0,   0.575, 0.0,
        0.0,   0.0,   0.575,
    ]);

    for col in 0..3 {
        assert_dvec3_near(inertia.col(col), expected_inertia.col(col), 1e-12);
    }
}
