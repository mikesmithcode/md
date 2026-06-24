use glam::{DVec3,DMat3};

use super::ParticleVec;

#[derive(Debug)]
pub struct MoleculeData{
    pub pids: Vec<usize>,
    pub inertia: DMat3, 
}

impl MoleculeData {
    pub fn new(pids: Vec<usize>, particles: &ParticleVec) -> Self {
        // Calculate CM
        let (_, com, _) = calculate_molecule_com(&pids, particles);
            
        // Calculate constant body-frame inertia tensor
        let inertia = calculate_molecule_inertia(&pids, particles, com);
        
        Self { pids, inertia}
    }

}


pub fn calculate_molecule_com(pids: &[usize], particles: &ParticleVec)-> (f64, DVec3, DVec3){
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

    return (total_mass, com_pos, vel)
}

pub fn calculate_molecule_inertia(pids: &[usize], particles: &ParticleVec, com: DVec3) -> DMat3 {
    let mut total_inertia = DMat3::ZERO;

    for &idx in pids {
        let m = particles.mass[idx];
        
        // Only massive particles contribute to inertia
        if m > 0.0 {
            let r = particles.position[idx] - com;
            let radius = particles.radius[idx];
            
            //Intrinsic inertia of a solid sphere: 2/5 * m * r^2
            let i_val = 0.4 * m * radius * radius;
            let i_local = DMat3::from_diagonal(DVec3::new(i_val, i_val, i_val));

            //Parallel Axis Theorem
            let r2 = r.dot(r);
            let outer_prod = DMat3::from_cols(
                DVec3::new(r.x * r.x, r.x * r.y, r.x * r.z),
                DVec3::new(r.y * r.x, r.y * r.y, r.y * r.z),
                DVec3::new(r.z * r.x, r.z * r.y, r.z * r.z),
            );
            
            let parallel_axis_shift = (DMat3::IDENTITY * r2) - outer_prod;
            
            total_inertia += i_local + (parallel_axis_shift * m);
        }
    }
    total_inertia
}



