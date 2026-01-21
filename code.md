# MD simulation in Rust

This is a simple molecular dynamics simulation written in Rust. It is intended as a learning project to explore Rust and scientific computing. 

Since the code is a workspace there are several crates:
- md_sim: the main simulation crate
- md_viz: the visualization crate
- md_utils: utility functions and types shared between crates

Different simulations are setup in the examples folder. To run a particular simulation use 
```bash
    cargo run --example <example_name>
```

## Graphics

Visualization is handled by md_viz crate.

My idea is that you should create a Scene struct which will control the graphics. This takes read only references to particles etc at intervals defined by the simulation loop. It then renders these.

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

Then in your simulation loop you can call `scene.display()` to update the window or `scene.save_img()` to save an image to file.

```rust
    scene.display(&simulation.get_particles()).expect("Error updating display");
```

### How are things drawn?

1. an immutable reference to a Vec of Particles is passed to scene.display() or scene.save_img() and then internally to self.
render_particles_to_target(). 

2. Each Particle struct has the Draw trait implemented for it which has a .draw() method. This takes a mutable reference to the objects vec in Scene. It uses the primitives defined templates e.g. sphere and adds to the vec. 

3. Other objects like lights and camera are also added to the objects vec in Scene.

4. The scene is rendered using the three_d crate.

## Data input / output

Eventually, I want to have a compiled program that takes a config file and an initial state file as input. The config file will define the simulation parameters. The initial state file will define the starting positions and velocities of the particles.

Periodically the simulation should write output files containing the current state of the system. These files can then be used for analysis or visualization. The same file should also be usable as an input file to restart a simulation. The state file needs to store the current timestep as well as the positions and velocities of all particles. I'd like to use polars for this. 

## Simulation loop

sim.update()
