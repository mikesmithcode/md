use glam::DVec3;
use three_d::core::Srgba;
use md::md_sim::particle::{Particle, ParticleVec};
use md::md_sim::file_io;
use std::path::Path;

pub fn main() {

let output_dir = "output/snapshots";

let mut particles = ParticleVec::new();
particles.push(Particle::new(
            0,
            0,
            DVec3::new(2.5, 0.05, 4.0),
            DVec3::new(0.0, 0.0, 0.0),
            0.5,
            3000.0,
            Srgba::new(255, 0, 0, 255), 
        ));
particles.push(Particle::new(
            1,
            1,
            DVec3::new(2.5, 0.05, 0.5),
            DVec3::new(0.0, 0.0, 0.0),
            0.5,
            3000.0,
            Srgba::new(0, 255, 0, 255), // Green
        ));

    file_io::save_snapshot(Path::new(output_dir),0,&particles,0.0).expect("Failed to save initial snapshot");

}
