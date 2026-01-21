use glam::DVec3;
use three_d::core::Srgba;
use md_core::particle::Particle;
use md_sim::file_io;
use std::path::Path;

pub fn main() {

let output_dir = "output/snapshots";

let particles = vec![
        Particle::new(
            0,
            DVec3::new(1.5, 1.5, 1.5),
            DVec3::new(0.0, 0.02, 0.0),
            Srgba::new(255, 0, 0, 255), // Red
            0.5,
        ),
        Particle::new(
            1,
            DVec3::new(1.5, -1.5, -1.5),
            DVec3::new(0.0, 0.003, 0.0),
            Srgba::new(0, 255, 0, 255), // Green
            0.5,
        ),
        Particle::new(
            2,
            DVec3::new(1.5, -1.5, 1.5),
            DVec3::new(0.0, 0.01, 0.0),
            Srgba::new(0, 0, 255, 255), // Blue
            0.5,
        ),
    ];

    file_io::save_snapshot(Path::new(output_dir),0,&particles,0.0).expect("Failed to save initial snapshot");

}
