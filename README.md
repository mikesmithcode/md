# MD simulation in Rust

This is a molecular dynamics simulation written in Rust. It is intended as a learning project to explore using Rust in scientific computing. 

Since the code is in a workspace there are two top-level folders:
- [`md_sim`]: the main simulation code
- [`md_viz`]: the visualization code


## Examples

Different simulations are contained in the bin folder.

If you simulation is in new_sim.rs. You can then run it with:
```bash
    cargo run --bin new_sim
```

In outline, you will create a new Simulation struct. Optionally decide on graphics via a new Scene struct. Then run a simulation loop calling sim.update() at each step. You can also write to file the results of your simulation and update the simulation time.

```rust
    let mut sim: Simulation = Simulation::new(sim_settings.clone());
    let mut scene: Scene = Scene::new(scene_settings.clone());

    for step in 0..sim_settings.num_steps {
        sim.update();
        scene.display(&simulation.get_particles(), step).expect("Error updating display");
        //save a snapshot
        file_io::save_snapshot(&snapshot_path, step, &simulation.get_particles(), time).expect("Error saving simulation snapshot");
        time += sim_settings.dt;
    }
```

## Simulation loop

sim.update() advances the simulation by one timestep. When you call sim.update() in the main loop this internally calls SimUpdate.update_motion(), then SimUpdate.update_forces(), then SimUpdate.correct_Motion() which are explained below. Need to think about what integration scheme you are using and apply things accordingly.

## Specifying the details of the simulation

To handle the details of a simulation you must create a unit struct (I've called mine SimUpdate) and then implement functions for two traits on it:

```rust

    struct SimUpdate;
    
    impl Motion for SimDetails{
        impl Motion for SimUpdate{
            fn update_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
                //Integrate the equations of motion.
                integrate_verler(forces, particles, settings);

                //You can also add anything else that modifies the data in particles eg. change the size of the particles.
            }

            fn correct_motion(&self, forces: &[glam::DVec3], particles: &mut ParticleVec,settings: &SimulationSettings) {
                integrate_verler(forces, particles, settings);
            }
        }    

    }

    impl Forces for SimUpdate{
        fn update_forces(&self, forces: &mut [glam::DVec3], particles: &ParticleVec, settings: &SimulationSettings) {
            // In here you define all the functions that require forces to be applied to particles.
            // forces.rs has a load of pre-built functions that you can import and use. 
            
            // If the force acts on all particles then pass the mutable reference to forces and a immutable reference to particles.
            //just call the function like this:
            add_weight(forces, particles);

            // If you need to calculate things such as collisions where you need to loop over particle pairs
            //Forces between particles - starting with checking all pairs.
            let n=particles.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    if let SimulationModel::Default(collision_params) = &settings.model{
                        inelastic_collision(i, j, particles, forces, collision_params);
                    }
                }
            }

            // If you have particles that shouldn't move or move with a specified motion put these last
            zero_forces_ptype(forces, particles, 1);
        }
    }

    
```

## Graphics

Visualization is handled by md_viz.

You should create a Scene struct which will control the graphics. This takes read only references to particles etc at intervals defined by the simulation loop. It then renders these.

```rust
    let mut scene: Scene = Scene::new(scene_settings.clone());
```

If you want a live window create an event loop and pass it to the init_window method:
   
```rust
    let mut event_loop = EventLoop::new(); 
    let _ = scene.init_window(&event_loop);
```

If you want headless rendering to an image file:
```rust
    let _ = scene.init_headless();
```

Then in your simulation loop you can call either `scene.display()` to update the window or `scene.save_img()` to save an image to file.

```rust
    scene.display(&simulation.get_particles(), step).expect("Error updating display");
    scene.save_img(&simulation.get_particles(),  &OUTPUT_PATH, step).expect("Error saving image");
```

scene.save_img will create a folder called output/imgs and save all imgs numbered by the simulation step. You can run the example [`create_video`] to convert these to a video using ffmpeg.

## Data input / output

A json config file defines the simulation parameters and an initial state file  defines the starting positions and velocities etc of the particles. The simulation parameters are used to construct a SimSettings struct. If you are using graphics there is also a SceneSettings struct read from a separate config file. The initial state file is a parquet file which can be read from polars. Each particle is a Particle struct which is stored in a Vec<Particle> in the Simulation struct.

Periodically the simulation should write output files containing the current state of the system. These files can then be used for analysis or visualization. The same file can also be used as an input file to restart a simulation. Restarting a simulation looks for the latest file in the output folder and uses that as the input file. This is why the simsettings specifies the number of steps to advance the simulation rather than a start and stop.


