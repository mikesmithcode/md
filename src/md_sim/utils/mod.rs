
pub mod file_io;
pub mod test_utils;

use glam::DVec3;

pub use file_io::{filepaths, save_sim_settings, load_sim_settings, save_particles, load_latest_particles, load_particles, load_scene_settings, load_latest_objects, save_objects, SimulationPaths};
pub use test_utils::{assert_dvec3_near,create_molecule_vec, create_single_molecule, create_particle_vec, setup_single_molecule_data, create_grid_and_settings};

#[cfg(test)]
mod tests;


// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
//
// Utility functions
//
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------


/// Applies the Minimum Image Convention to a displacement vector.
///
/// In a periodic simulation box, a particle can interact with either the real 
/// instance of another particle or its periodic images. This function adjusts 
/// the displacement vector `delta` to ensure it represents the shortest 
/// distance between two points across the periodic boundaries.
///
/// # Arguments
///
/// * `delta` - The mutable displacement vector ($r_j - r_i$) to be wrapped.
/// * `sim_box_size` - The dimensions of the periodic simulation cell.
///
/// # Physics Context
///
/// If a component of the displacement exceeds half the box length, the 
/// nearest image is actually in the opposite direction (through the boundary).
/// For a box of length $L$, if $\Delta x > L/2$, the shortest path is $\Delta x - L$.
///
/// # Notes
///
/// * This function is essential for correct force calculations and collision 
///   handling in periodic systems.
/// * It assumes the initial displacement was calculated using coordinates 
///   already mapped (or "wrapped") within the primary simulation box.
pub fn check_delta(delta: &mut DVec3, sim_box_size: DVec3,periodic:[bool;3]) {
    // Check X-axis wrapping
    if periodic[0]{
        if delta.x > sim_box_size.x * 0.5 { 
            delta.x -= sim_box_size.x; 
        } else if delta.x < -sim_box_size.x * 0.5 { 
            delta.x += sim_box_size.x; 
        }
    }

    // Check Y-axis wrapping
    if periodic[1]{
        if delta.y > sim_box_size.y * 0.5 { 
            delta.y -= sim_box_size.y; 
        } else if delta.y < -sim_box_size.y * 0.5 { 
            delta.y += sim_box_size.y; 
        }
    }

    // Check Z-axis wrapping
    if periodic[2]{
        if delta.z > sim_box_size.z * 0.5 { 
            delta.z -= sim_box_size.z; 
        } else if delta.z < -sim_box_size.z * 0.5 { 
            delta.z += sim_box_size.z; 
        }
    }
}


/// Context and parameters required for neighbor searches and pairwise particle interactions.
///
/// # Fields
/// * `sim_box_size` - The physical dimensions of the periodic simulation boundary box $(L_x, L_y, L_z)$.
/// * `periodic` - Boolean flags specifying periodicity along each of the three Cartesian axes `[x, y, z]`.
/// * `search_radius_sq` - Pre-squared neighbor cutoff search radius to avoid expensive square root operations during distance checks.
/// * `interaction_ptypes` - Slice of allowed particle type pairs `[type_a, type_b]` that evaluate interaction forces.
pub struct InteractionContext<'a> {
    pub sim_box_size: DVec3,
    pub periodic: [bool; 3],
    pub search_radius_sq: f64,
    pub interaction_ptypes: &'a [[u8; 2]],
}
