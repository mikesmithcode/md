# Overview of how the code works

## Setup

A simulation needs 3 config files:
- scene --> Controls graphics parameters e.g.

```json
  {
    "camera": {
      "cam_type": "Perspective",  //"Perspective or Orthographic
      "fov": 35.0,  // angle of field of view
      "rel_pos": [0.0, -0.2, 0.0],    // camera position relative to centre
      "up":[0.0, 0.0, 1.0]   // definition of up 
      },
    "window_size": [1280, 960],
    "window_visible":true,
    "display_steps": 100,  // frequency of refresh
    "vid_fps": 30,
    "sim_box": {
      "visible": true,
      "thickness": 0.0005
    }
  }

```

- sim --> Controls details of simulation

```json
{
  "dt": 1.0e-5, // simulation timestep
  "sim_box_size": [0.02, 0.02, 0.04],  //sim box dimensions [x,y,z]
  "periodic": [true,true,false],  // boundary conditions [x,y,z]
  "dimensions": "XYZ", // enum specifying 3D XYZ or 2D XZ, XY or YZ
  "parallel":true,  // uses parallel processing on pairwise forces
  "threads":0,   // max number of threads to use. 0 means let Rayon decide.
  "skin": 0.0002,  // the distance travelled by the furthest moving particle at which the CellGrid and Verlet lists are rebuilt
  "start": 0,  // Initial step value of simulation
  "num_steps": 2000000,   // How many steps to run before stopping
  "dump": 1000,   // interval in steps at which to store particle positions etc
  "interaction_ptypes": [[0,0,0.002],[0,1,0.002]],   // [i,j,cutoff] defines whether j ptype will appear in i ptype's verlet list. Limited by cutoff. Single directional. i won't necessarily appear in js list.
  "collision_ptypes":[0,1]  // pytpe of particles which undergo collisions.
}

```


- model --> This contains a section for every force law and the relevant parameters e.g.

```json
{
 "normal_force": {
    "modulus": 1.0e5,
    "restitution": 0.7,
    "plane_modulus": 1.0e6,
    "plane_restitution": 0.7
  },
  "friction":{
    "modulus":1.0e5,
    "mu":0.4,
    "restitution":0.7
  },
  "gravity": {
    "g" : -9.81,
    "up": [0.0,0.0,1.0]    
  }
}
```

The user defines a [`SimUpdate`] struct which holds the ForceModel derived from the model json a ContactManager used to trace contacts between particles. A SimulationContext which holds paths to relevant folders in a SimulationPaths and various bool options to record video etc.

```Rust
#[derive(Clone, Debug, Serialize)]
pub struct SimUpdate {
    pub model: ForceModel,
    #[serde(skip)]
    pub contact_manager: ContactManager,
    pub ctx: SimulationContext,
}
```

## Implementing traits

You implement 3 traits on this Struct: Force, Motion, Interactivity

### Force

Force defines forces in three passes. Single particle forces, object forces and pairwise forces.

```Rust
fn update_single_forces(&self, _i: usize, force: DVec3, torque: DVec3,_particles: &ParticleVec, _settings: &SimulationSettings,_time: f64) -> (DVec3, DVec3) {
        (force, torque)
    }

fn update_pair_forces(&self, _i: usize, _j: usize, force: DVec3, torque: DVec3, _particles: &ParticleVec, _settings: &SimulationSettings) -> (DVec3, DVec3) {
        (force, torque)
    }

fn update_object_forces(&self, _i: usize, force: DVec3, torque: DVec3,_particles: &ParticleVec, _objects: &ObjectSpec,_settings: &SimulationSettings) -> (DVec3, DVec3) {
        (force, torque)
    }
```

These all work at a single particle level. ie you don't need to worry about the mechanics. 



