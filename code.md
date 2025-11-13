## Graphics

Visualization is handled by md_viz crate.

My idea is that you should create a Scene struct which will control the graphics.

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



