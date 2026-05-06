# MD simulation in Rust

This is a molecular dynamics simulation written in Rust. It is intended as a learning project to explore using Rust in scientific computing. 

There are two top-level folders:
- [`md_sim`]: the main simulation code
- [`md_viz`]: the visualization code

In addition you have the bin folder where the entry points for different simulations are stored. See an explanation of [different simulation](crate::simulations).

To make things easier this is a guide of both cargo commands and my own bash script commands to run things [Commands](crate::cmds) and some [know how](crate::knowhow).


## Outline of a simulation

In outline, inside your simulation script which lives in src/bin you will create a new Simulation struct. Optionally decide on graphics via a new Scene struct. Then run a simulation loop calling sim.update() at each step. You can also write to file the results of your simulation and update the simulation time.

Open the template simulation in to see a skeleton of how to set up a simulation. This file contains detailed comments on how to set up the simulation, the simulation loop, the SimUpdate struct which defines the details of the simulation and the Scene struct which defines the graphics.

```rust, ignore
    let mut sim: Simulation = Simulation::new(sim_settings.clone());
    let mut scene: Scene = Scene::new(scene_settings.clone());

    for step in 0..sim_settings.num_steps {
        sim.update();
        scene.display(&simulation.get_particles()).expect("Error updating display");
        //save a snapshot
        file_io::save_snapshot(&snapshot_path, &simulation.get_particles(), time).expect("Error saving simulation snapshot");
        time += sim_settings.dt;
    }
```

## Simulation loop

sim.update() advances the simulation by one timestep. When you call sim.update() in the main loop this internally calls SimUpdate.update_motion(), then SimUpdate.update_forces(), then SimUpdate.correct_Motion() which are explained below. Need to think about what integration scheme you are using and apply things accordingly.

## Specifying the details of the simulation

To handle the details of a simulation you must create a unit struct (I've called mine SimUpdate) and then implement functions for two traits on it:

```rust, ignore

    struct SimUpdate;
    
    impl Motion for SimUpdate{
        fn update_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
            //Integrate the equations of motion.
            integrate_verler_update(forces, particles, settings);

            //You can also add anything else that modifies the data in particles eg. change the size of the particles.
        }

        fn correct_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
            integrate_verler_correct(forces, particles, settings);
        }
    }    

    

    impl Forces for SimUpdate{
        
        fn has_pair_forces(&self) -> bool { false }
            // If there are no pair forces then call this method returning false. If you do have pair forces this defaults to true and you 
            // need not implement it.
        
        fn update_pair_forces(&self,i: usize,j: usize,forces: &mut [DVec3],particles: &ParticleVec,settings: &SimulationSettings); {
            // In here you define all the functions that require forces to be applied between pairs of particles.
            // forces.rs has a load of pre-built functions that you can import and use. The Simulation handles finding 
            // particle pairs according to the cutoff distance and iterates over them. i and j are indices which specify the pair of particles.
            // You then calculate the interaction between these particles and update the forces on each particle accordingly.
            inelastic_collision(i, j, particles, forces, collision_params);
        }

        fn has_single_forces(&self) -> bool { false }
            // If there are no single particle forces then call this method returning false. If you do have single particle forces this defaults to true and you 
            // need not implement it.

        fn update_single_forces(&self,i:usize, forces: &mut [glam::DVec3], particles: &ParticleVec, _settings: &SimulationSettings, time: f64) {
            // In here you define all the functions that require forces to be applied to particles.
            // forces.rs has a load of pre-built functions that you can import and use. 
            
            // If the force acts on all particles then pass the mutable reference to forces and an immutable reference to particles.
            //just call the function like this:
            add_weight(forces, particles);

            // If you need to calculate things such as collisions where you need to loop over particle pairs
            //Forces between particles - starting with checking all pairs.
        }                    
        
        // For particles that shouldn't follow the calculated forces e.g walls etc.
        fn update_ptype_no_forces(&self, forces: &mut [DVec3], particles: &ParticleVec){
            let immobile = &[1, 2];
            zero_forces_for_ptypes(forces, particles, immobile);
        }
    }
    
    
```

## Graphics

Visualization is handled by md_viz. This uses [open_gl graphics](crate::opengl)

You should create a Scene struct which will control the graphics. This takes read only references to particles etc at intervals defined by the simulation loop. It then renders these.

```rust, ignore
    let mut scene: Scene = Scene::new(scene_config_path, &sim_settings);
```

If you want a live window create an event loop and pass it to the init_window method:
   
```rust, ignore
    let mut event_loop = EventLoop::new(); 
    let _ = scene.view(&event_loop);
    let _ = scene.start_recording(&video_path, start_step);
```
If you don't need a live window but want to record images to a video you should instead use

```rust, ignore
    let mut event_loop = EventLoop::new(); 
    let _ = scene.background(&event_loop);
    let _ = scene.start_recording(&video_path, start_step);
```

Then in your simulation loop you can call either [`md_viz::scene::Scene::display`] to update the window or [`md_viz::scene::Scene::save_frame`] to save a frame to the video.

```rust, ignore
    scene.display(&simulation.get_particles(), step).expect("Error updating display");
    scene.save_frame(&simulation.get_particles(),  &OUTPUT_PATH, step).expect("Error saving image");
```

scene.save_frame pushes each displayed frame to an ffmpeg video stream which appears in output/sim_name.

Alternatively, you can just record data output during the simulation and later construct a video using the [`video`](src/bin/video.rs) script. Here you simply supply the simulation name as a command line argument. ie `run video sim_name` and it will construct a video based on the parquet output files in output/sim_name.

## Data input / output

A json config file (in input) defines the simulation parameters and an initial state file (.parquet in output) defines the starting positions and velocities etc of the particles. The simulation parameters are used to construct a SimSettings struct. If you are using graphics there is also a SceneSettings struct read from a separate config file. The initial state file is a parquet file which can be read from polars. Each particle is a Particle struct which is stored in a `Vec<Particle>` in the Simulation struct. The parquet file is either the output of a previous simulation or constructed using a python script.

Periodically the simulation should write output files containing the current state of the system. These files can then be used for analysis or visualization. The same file can also be used as an input file to restart a simulation. Restarting a simulation looks for the latest file in the output folder and uses that as the input file. This is why the simsettings specifies the number of steps to advance the simulation rather than a start and stop.

## Simulation

Simulation has two main methods: [`md_sim::simulation::Simulation::new`] which creates a new simulation and [`md_sim::simulation::Simulation::update`] which advances the simulation by one step.

[`md_sim::simulation::Simulation::new`]
This takes a SimSettings struct as input and creates a new Simulation struct. It reads the initial state from file and creates the cell grid for efficient finding of neighbouring particles. It also takes a SimUpdate unit struct which contains the details of the simulation. This implements the traits [`md_sim::motion::Motion`] and [`md_sim::force::Forces`]. Each of these traits needs to be implemented for your specific simulation. You can use the pre-built functions in motion.rs and forces.rs or write your own.

[`md_sim::motion::Motion`] defines the [`md_sim::motion::Motion::update_motion`] and [`md_sim::motion::Motion::correct_motion`] functions. [`md_sim::motion::Motion::update_motion`] is where you would implement your integration scheme to update the positions and velocities of the particles based on the forces. [`md_sim::motion::Motion::correct_motion`] is where you would implement any correction step of your integration scheme if you are using one. This method is optional and can be left blank.

[`md_sim::force::Forces`] defines the [`md_sim::force::Forces::update_single_forces`] function which is for those forces which act on individual particles. It also defines the [`md_sim::force::Forces::update_pair_forces`] function which is for forces that act between pairs of particles. If you don't want to use either you must reimplement [`md_sim::force::Forces::has_pair_forces`] or [`md_sim::force::Forces::update_single_forces`] returning false to tell the simulation you don't need these. the The CellGrid structure efficiently searches for particle pairs within a specified cutoff distance defined in your SimSettings, read from a config file.

[`Simulation::update`]
This works through several steps:
- update_motion() this provides an initial prediction of the new particle positions and velocities based on the current forces. This is where you would implement your integration scheme.
- update_forces() this calculates the forces on the particles based on their new positions. This is where you would implement the physics of your system. You can use the pre-built functions in forces.rs or write your own.
- correct_motion() this corrects the particle positions and velocities based on the new forces. This is where you would implement the correction step of your integration scheme if you are using one. This method is optional and can be left blank.

Simulation creates a [`md_sim::neighbours::CellGrid`] structure. This is a grid which bins the particles based on their positions. For each particle we then create a verlet list of particles within some distance cutoff + skin. This is efficient because it is only searching the same cell or neighbouring cells. When any particle has undergone a displacement equal to skin we rebuild the cell grid and verlet lists.
