# MD simulation in Rust

This is a molecular dynamics simulation written in Rust. It is intended as a learning project to explore using Rust in scientific computing. 

Since the code is in a workspace there are several crates:
- [`md_sim`]: the main simulation crate
- [`md_viz`]: the visualization crate
- [`md_core`]: utility functions and types shared between crates

## Examples

Different simulations are contained in the example folder. Each new simulation needs to be referenced in the md_viz Cargo.toml file:
```bash
[[example]]
name = "new_sim"
path = "../../examples/new_sim.rs"
```

This defines a new example called new_sim. You can then run it with:
```bash
    cargo run --example new_sim
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

## Graphics

Visualization is handled by the md_viz crate.

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

## Simulation loop

sim.update()
