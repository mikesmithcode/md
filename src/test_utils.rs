    use three_d::core::Srgba;
    use glam::DVec3;
    use crate::md_sim::particle::Particle;
    use crate::md_sim::particle::ParticleVec;

    const NULL_ID: usize = usize::MAX;

    pub fn create_particle_vec()-> ParticleVec{
        let mut particles = ParticleVec::new();
        particles.push(
            Particle {
                id: 0,
                next_id: NULL_ID,
                ptype: 0,
                position: DVec3::new(1.0, 2.0, 3.0),
                rel_pos: DVec3::ZERO,
                velocity: DVec3::new(1.0, 1.0, 1.0),
                orientation: DVec3::new(1.0,0.0,0.0),
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
                next_id: NULL_ID,
                ptype: 1,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                rel_pos: DVec3::ZERO,
                orientation: DVec3::new(1.0,0.0,0.0),
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
