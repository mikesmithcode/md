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
