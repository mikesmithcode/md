#![doc = include_str!("../README.md")]
/// Documentation regarding the physics engine logic
pub mod simulations {
    #![doc = include_str!("../docs/simulations/Integration.md")]
    #![doc = include_str!("../docs/simulations/MulticomponentParticles.md")]
    #![doc = include_str!("../docs/simulations/Quaternions.md")]
    #![doc = include_str!("../docs/simulations/Simulations.md")]
}

/// Information on how to use the CLI or config files
pub mod knowhow {
    #![doc = include_str!("../docs/knowhow.md")]
}

/// Information on how to use the CLI or config files
pub mod opengl {
    #![doc = include_str!("../docs/opengl.md")]
}

/// Information on how to use the CLI or config files
pub mod cmds {
    #![doc = include_str!("../docs/cmds.md")]
}


pub mod md_viz;
pub mod md_sim;

pub use crate::md_sim::particle::*;
pub use crate::md_sim::simulation::*;

#[cfg(test)]
pub mod test_utils;
