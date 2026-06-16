    use three_d::core::Srgba;
    use glam::{DVec3, DQuat};
    use std::collections::HashMap;

    use crate::md_sim::particle::Particle;
    use crate::md_sim::particle::ParticleVec;
    use crate::md_sim::motion::geometry::MoleculeData;

    const NULL_ID: usize = usize::MAX;

    pub fn create_particle_vec()-> ParticleVec{
        let mut particles = ParticleVec::new();
        particles.push(
            Particle {
                id: 0,
                molecule_id: 0,
                ptype: 0,
                position: DVec3::new(1.0, 2.0, 3.0),
                rel_pos: DVec3::ZERO,
                velocity: DVec3::new(1.0, 1.0, 1.0),
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                inertia: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
        particles.push(
            Particle {
                id: 1,
                molecule_id: 1,
                ptype: 1,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                inertia: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });

            particles
    }

pub fn create_molecule_vec() -> ParticleVec {
    let mut particles = ParticleVec::new();
    
    let com = DVec3::new(1.0, 2.0, 3.25);
    
    // Particle 0 is at (1.0, 2.0, 3.5) -> rel_pos = (0, 0, 0.5)
    particles.push(Particle {
        id: 0,
        molecule_id: 0,
        ptype: 0,
        position: com + DVec3::new(0.0, 0.0, 0.5),
        rel_pos: DVec3::new(0.0, 0.0, 0.25),
        velocity: DVec3::new(1.0, 1.0, 1.0), // v_com + (omega x r0)
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 1.5,
        inertia: 1.0,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });
    
    // Particle 1 is at (1.0, 2.0, 2.5) -> rel_pos = (0, 0, -0.5)
    particles.push(Particle {
        id: 1,
        molecule_id: 0,
        ptype: 1,
        position: com + DVec3::new(0.0, 0.0, -0.5),
        rel_pos: DVec3::new(0.0, 0.0, -0.75),
        velocity: DVec3::new(0.0, 1.0, 1.0),
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 0.5,
        inertia: 1.0,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });

    particles
}

pub fn setup_single_molecule_data(particles: &ParticleVec) -> HashMap<usize, MoleculeData> {
    let mut map = HashMap::new();
    
    // We define the molecule as containing particles 0 and 1
    let pids = vec![0, 1];
    
    // The 'new' method automatically calculates COM and the inertia tensor 
    // based on the current positions in the ParticleVec
    let mol_data = MoleculeData::new(pids, particles);
    
    // Map molecule ID 0 to this data
    map.insert(0, mol_data);
    
    map
}