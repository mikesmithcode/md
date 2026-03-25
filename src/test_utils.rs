    use three_d::core::Srgba;
    use glam::DVec3;
    use crate::md_sim::particle::Particle;
    use crate::md_sim::particle::ParticleVec;

    pub fn create_particle_vec()-> ParticleVec{
        let mut particles = ParticleVec::new();
        particles.push(
            Particle {
                id: 0,
                ptype: 0,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(1.0, 1.0, 1.0),
                radius: 0.5,
                mass: 1.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });
        particles.push(
            Particle {
                id: 1,
                ptype: 1,
                position: DVec3::new(1.0, 2.0, 3.0),
                velocity: DVec3::new(0.1, 0.2, 0.3),
                radius: 0.5,
                mass: 1.0,
                color: Srgba::new(255, 0, 0, 255),
                ref_pos: DVec3::ZERO,
            });

            particles
    }
