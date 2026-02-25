//! draw_particles.rs
//!
//! This module defines the Draw trait for [`Particle`]. This specifies how to render a single
//! particle. We define a default implementation for a sphere but if you have more complicated
//! shapes you can define these. 
//! 
//! get_components() is called by scene::Scene::render_particles() on each particle to render it.
//!  
//!
//! 



use three_d::{
    Mat4
};
use md_core::particle::Particle;
use crate::templates::{Geometry, ParticleInstanceData};


pub trait Draw {
    ///Defines all the objects within a single particle to be drawn and returns to be rendered by Scene.render_particles_to_target
    fn get_components(&self)-> Result<Vec<ParticleInstanceData>, Box<dyn std::error::Error>>;
}

impl Draw for Particle{
    ///This is for a single sphere
    fn get_components(&self)-> Result<Vec<ParticleInstanceData>, Box<dyn std::error::Error>>{
    // Calculate the single transformation
        let transformation = Mat4::from_translation(three_d::vec3(
            self.position.x as f32,
            self.position.y as f32,
            self.position.z as f32,
        )) * Mat4::from_scale(self.radius as f32);

        // Return a Vec containing data for a single instance
        Ok(vec![ParticleInstanceData {
            template: Geometry::Sphere,
            transformation,
            color: self.color,
        }])
    }
}


//Tests
#[cfg(test)]
mod tests {
    use super::*;
    use md_core::particle::Particle;
    use glam::DVec3;
    use three_d::Srgba;

    #[test]
    fn test_particle_draw_returns_single_component() {
        let particle = Particle::new(
            1,
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 0.0, 0.0),
            Srgba::new(255, 0, 0, 255),
            0.5,
        );

        let result = particle.get_components();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_particle_draw_correct_geometry() {
        let particle = Particle::new(
            1,
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 0.0),
            Srgba::new(255, 0, 0, 255),
            1.0,
        );

        let components = particle.get_components().unwrap();
        assert_eq!(components[0].template, Geometry::Sphere);
    }
}
