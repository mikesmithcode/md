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



