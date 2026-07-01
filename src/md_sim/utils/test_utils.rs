    use three_d::core::Srgba;
    use glam::{DVec3, DQuat};
    use std::collections::HashMap;

    use crate::md_sim::force::CellGrid;
    use crate::md_sim::SimulationSettings;
    use crate::md_sim::particle::{Particle, ParticleVec, MoleculeData};

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
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
        particles.push(
            Particle {
                id: 2,
                molecule_id: 1,
                ptype: 0,
                position: DVec3::new(8.0, 8.0, 8.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
        particles.push(
            Particle {
                id: 3,
                molecule_id: 1,
                ptype: 2,
                position: DVec3::new(8.0, 8.0, 5.5),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
            particles.push(
            Particle {
                id: 4,
                molecule_id: 1,
                ptype: 2,
                position: DVec3::new(8.0, 8.0, 4.5),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
            particles.push(
            Particle {
                id: 5,
                molecule_id: 1,
                ptype: 2,
                position: DVec3::new(5.0, 5.0, 5.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DQuat::IDENTITY,
                omega: DVec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
                mass: 1.0,
                charge: 0.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
      

            particles
    }

pub fn create_single_molecule()-> ParticleVec{
    let mut particles = ParticleVec::new();
    
    let com = DVec3::new(1.0, 2.0, 3.25);
    
    // Particle 0 is at (1.0, 2.0, 3.5) -> rel_pos = (0, 0, 0.5)
    particles.push(Particle {
        id: 0,
        molecule_id: 0,
        ptype: 0,
        position: com + DVec3::new(0.0, 0.0, 0.25),
        rel_pos: DVec3::new(0.0, 0.0, 0.25),
        velocity: DVec3::new(1.0, 1.0, 1.0), // v_com + (omega x r0)
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 0.0, 0.0),
        radius: 0.5,
        mass: 1.5,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });
    
    // Particle 1 is at (1.0, 2.0, 2.5) -> rel_pos = (0, 0, -0.5)
    particles.push(Particle {
        id: 1,
        molecule_id: 0,
        ptype: 1,
        position: com + DVec3::new(0.0, 0.0, -0.75),
        rel_pos: DVec3::new(0.0, 0.0, -0.75),
        velocity: DVec3::new(1.0, 1.0, 1.0),
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 0.0, 0.0),
        radius: 0.5,
        mass: 0.5,
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
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });



    let com = DVec3::new(2.0, 2.0, 3.25);
    // Particle 0 is at (1.0, 2.0, 3.5) -> rel_pos = (0, 0, 0.5)
    particles.push(Particle {
        id: 2,
        molecule_id: 1,
        ptype: 0,
        position: com + DVec3::new(0.0, 0.0, 0.5),
        rel_pos: DVec3::new(0.0, 0.0, 0.25),
        velocity: DVec3::new(1.0, 1.0, 1.0), // v_com + (omega x r0)
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 1.5,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });
    
    // Particle 1 is at (1.0, 2.0, 2.5) -> rel_pos = (0, 0, -0.5)
    particles.push(Particle {
        id: 3,
        molecule_id: 1,
        ptype: 1,
        position: com + DVec3::new(0.0, 0.0, -0.5),
        rel_pos: DVec3::new(0.0, 0.0, -0.75),
        velocity: DVec3::new(0.0, 1.0, 1.0),
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 0.5,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });

    let com = DVec3::new(8.0, 2.0, 3.25);
    // Particle 0 is at (1.0, 2.0, 3.5) -> rel_pos = (0, 0, 0.5)
    particles.push(Particle {
        id: 4,
        molecule_id: 2,
        ptype: 0,
        position: com + DVec3::new(0.0, 0.0, 0.5),
        rel_pos: DVec3::new(0.0, 0.0, 0.25),
        velocity: DVec3::new(1.0, 1.0, 1.0), // v_com + (omega x r0)
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 1.5,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });
    
    // Particle 1 is at (1.0, 2.0, 2.5) -> rel_pos = (0, 0, -0.5)
    particles.push(Particle {
        id: 5,
        molecule_id: 2,
        ptype: 1,
        position: com + DVec3::new(0.0, 0.0, -0.5),
        rel_pos: DVec3::new(0.0, 0.0, -0.75),
        velocity: DVec3::new(0.0, 1.0, 1.0),
        orientation: DQuat::IDENTITY,
        omega: DVec3::new(0.0, 1.0, 0.0),
        radius: 0.5,
        mass: 0.5,
        charge: 0.0,
        color: Srgba::new(255, 0, 0, 255),
        ref_pos: DVec3::ZERO,
    });


    particles
}

pub fn setup_single_molecule_data(particles: &ParticleVec) -> HashMap<usize, MoleculeData> {
    let mut map = HashMap::new();
    
    // molecule contains 2 particles
    let pids = vec![0, 1];
    
    let mol_data = MoleculeData::new(pids, particles);
    
    // Map molecule ID 0 to this data
    map.insert(0, mol_data);
    
    map
}

pub fn create_grid_and_settings()->(CellGrid, SimulationSettings){
    
    let particle_count = 6;
    let settings = SimulationSettings {
            cutoff:2.8,
            skin: 0.2,
            sim_box_size: DVec3::splat(9.0),
            periodic:[true;3],
            interaction_ptypes: vec![[0,1]],
            ..Default::default()
        };

    let grid = CellGrid::new(particle_count, &settings);
    (grid, settings)
}
