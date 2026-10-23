/// Particles composed of chargeable patches bouncing in a box.


use glam::DVec3;
use serde::{Serialize, Deserialize};

// Imports from simulation library
//traits
use md::md_sim::{Forces, Motion, InputOutput};
//high level simulation structures
use md::md_sim::{ObjectSpec, ParticleVec, SimulationSettings};
use md::md_sim::utils::file_io::SimulationContext;
use md::md_sim::MoleculeMap;
//model structs
use md::md_sim::motion::Vibration;
use md::md_sim::particle::Charging;
use md::md_sim::force::{FrictionCundallStrack, Gravity, NormalForce};
//contact managements
use md::md_sim::force::{ContactManager, ContactType};
// functions
use md::md_sim::force::{add_gravity, normal_linear, friction_cundall_strack};
use md::md_sim::motion::{integrate_rigid_bodies, integrate_rigid_bodies_correct};
use md::md_sim::particle::find_closest_charge;
use md::md_sim::force::add_charge;
use md::md_sim::utils::file_io::parse_simulation_args;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub normal_force: NormalForce,
    pub friction: FrictionCundallStrack,
    pub gravity: Gravity,
    pub vibration: Vibration,
    pub charging: Charging,
}

#[derive(Clone, Debug, Serialize)]
pub struct SimUpdate {
    pub model: Model,
    #[serde(skip)]
    pub contact_manager: ContactManager,
    pub ctx: SimulationContext,
}

impl SimUpdate {
    /// Initializes a new simulation update context by loading the latest force model 
    /// configuration and restoring prior contact states if available.
    pub fn new(ctx: SimulationContext) -> Self {
        // Load the latest model config from the output config directory using SimulationContext
        let model: Model = ctx.load_model().expect("Error loading model config from output/config");

        // Attempt to load the latest saved contact state; fallback to a fresh manager if none exists
        let contact_manager = ContactManager::load_latest(&ctx)
            .unwrap_or_else(|| {
                println!("New ContactManager Created");
                ContactManager::new()
            });

        Self { model, contact_manager, ctx }
    }
}



impl Forces for SimUpdate{
    // Default implementation is true, set to false if not using
    fn has_pair_forces(&self)-> bool {
        true
    }
    // Default implementation is true set to false if not using
    fn has_single_forces(&self)-> bool {
        true
    }
    //Default implementation is true set to false if not using
    fn has_object_forces(&self) -> bool {
        true
    }

    
    fn handle_contact_separation(
        &mut self, particles: &mut ParticleVec, 
        molecule_map: &MoleculeMap,
        _settings: &SimulationSettings
    ) {
         self.contact_manager.prune_contacts(|contact_type, contact| {
             // `contact_type` tells you which two particles or particle-plane were in contact.
             // `state` gives you the last known state of that contact.

             //NEED TO ADD CORRECT CHARGING LOGIC IN HERE
             match *contact_type{
                ContactType::Particle{i,j} =>{
                    
                    let i_q = find_closest_charge(i, contact, particles, molecule_map);
                    add_charge(i_q, particles, self.model.charging.dq_pp, self.model.charging.q_max);
                    
                    let j_q = find_closest_charge(j, contact, particles, molecule_map);
                    add_charge(j_q, particles, self.model.charging.dq_pp, self.model.charging.q_max);

                    //println!("Contact between particle {} and {} ended;", i, j);
                    //println!("particles have ptypes {} and {} ended;", particles.ptype[i], particles.ptype[j]);
                    //println!("Charges added to {} and {}", i_q, j_q);
                }
                ContactType::Plane{i, plane_id} =>{
                    let i_q = find_closest_charge(i, contact, particles, molecule_map);
                    add_charge(i_q, particles, self.model.charging.dq_ps, self.model.charging.q_max);
                    //println!("Contact between particle {} and plane {} ended; ", i, plane_id);
                    //println!("Charge on {} now = {}", i_q, &particles.charge[i_q]);

                }
             }
             
          });
    }


    //Forces which apply to every particle individually, e.g. gravity
    fn update_single_forces(&self,i:usize, mut force:glam::DVec3, _torque: DVec3, particles: &ParticleVec, _settings: &SimulationSettings, _time: f64)->(DVec3, DVec3) {   
        
        // Only the dynamic particle has weight
        if particles.ptype[i] == 0{
            
            force = add_gravity(i, force, particles, &self.model.gravity);
            //println!("add_gravity: particle id : {}, ptype {}, force {:?}", i, &particles.ptype[i], force);
        }
        (force, _torque)
    }

    /// Forces between particle i and j.
    /// 
    /// Only pairs of ptypes specified in sim.json with correct interaction_ptype will be passed
    /// to this fn. [i,j] doesn't guarantee [j,i]. 
    fn update_pair_forces(&self,i: usize,j: usize, mut force: DVec3, mut torque: DVec3, particles: &ParticleVec,settings: &SimulationSettings)->(DVec3, DVec3){
        //Only main particles have granular collisions.
        if let Some(mut contact) = self.contact_manager.analyse_particle_contact(i,j,particles, settings){
            //Normal and tangential forces
            let (fn_mag, fn_vec)=normal_linear(&contact, &self.model.normal_force);
            //let (ft_vec, tt_vec) = friction_viscous_damping(fn_mag, &contact, &self.model.friction);
            let (ft_vec, tt_vec) = friction_cundall_strack(fn_mag, &mut contact, &self.model.friction);

            //Add to accumulators
            force += fn_vec + ft_vec;
            torque += tt_vec;
        }
        (force, torque)
    
    }

    /// Forces between particle i and a single object
    /// 
    /// Only ptypes specified as collision_ptypes get passed. object_spec handles internally 
    /// what kind of object rect, triangle etc.
    fn update_object_forces(
        &self,
        i: usize,
        mut force: DVec3,
        mut torque: DVec3,
        particles: &ParticleVec,
        object_spec: &ObjectSpec,
        settings: &SimulationSettings
    ) -> (DVec3, DVec3) {
        if let Some(mut contact) = self.contact_manager.analyse_surface_contact(i,particles, object_spec, settings){
        
            let (fn_mag, fn_vec)=normal_linear(&contact, &self.model.normal_force);
            //let (ft_vec, tt_vec) = friction_viscous_damping(fn_mag, &contact, &self.model.friction);
            let (ft_vec, tt_vec) = friction_cundall_strack(fn_mag, &mut contact, &self.model.friction);

            //Add to accumulators
            force += fn_vec + ft_vec;
            torque += tt_vec;
        }

        (force, torque)
    }

}


impl Motion for SimUpdate{
    fn update_motion(&self, forces: &[glam::DVec3], torques: &[DVec3],particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &MoleculeMap, _time:f64) {
        integrate_rigid_bodies(forces,torques, particles, molecule_map, settings);
    }

    // Optional depending on integration scheme.
    fn correct_motion(&self, forces: &[glam::DVec3], torques: &[DVec3], particles: &mut ParticleVec,settings: &SimulationSettings, molecule_map: &MoleculeMap) {
        integrate_rigid_bodies_correct(forces, torques, particles, molecule_map, settings);

    }

    fn update_objects(&mut self, object: &mut ObjectSpec, _particles: &mut ParticleVec,settings: &SimulationSettings, time: f64){
        match object {
            ObjectSpec::Rectangle(rect) => {
                let ang_freq = self.model.vibration.ang_freq;
                let velocity = self.model.vibration.velocity*f64::sin(ang_freq*time);
                rect.step(velocity, DVec3::ZERO, settings.dt);
            },
            _ => {}   
        }

    }
}


impl InputOutput for SimUpdate {
    fn ctx(&self) -> &SimulationContext {
        &self.ctx
    }


    fn save_on_exit(&self, step: usize) -> Result<(), std::io::Error> {
        self.contact_manager.save_at_step(self.ctx(), step);
        self.ctx.save_model(&self.model, step)?;
        Ok(())
    }
}






pub fn main() {    
    let ctx = parse_simulation_args();
    let sim_update = SimUpdate::new(ctx);
    md::run_simulation(sim_update);
}
