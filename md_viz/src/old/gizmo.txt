// --- GIZMO AXES RENDERING ---
        /*let window_width = frame_input.viewport.width as f32;
        let window_height = frame_input.viewport.height as f32;

        let gizmo_viewport = Viewport {
            x: (window_width - self.gizmo.gizmo_size_pixels - self.gizmo.gizmo_padding_pixels) as i32,
            y: (window_height - self.gizmo.gizmo_size_pixels - self.gizmo.gizmo_padding_pixels) as i32,
            width: self.gizmo.gizmo_size_pixels as u32,
            height: self.gizmo.gizmo_size_pixels as u32,
        };
        self.gizmo.gizmo_camera.set_viewport(gizmo_viewport);

        let main_cam_world_transform: Mat4 = self.camera.view().invert().unwrap();

        let main_cam_rotation_only_transform = Matrix3::new(
            main_cam_world_transform.x.x, main_cam_world_transform.y.x, main_cam_world_transform.z.x,
            main_cam_world_transform.x.y, main_cam_world_transform.y.y, main_cam_world_transform.z.y,
            main_cam_world_transform.x.z, main_cam_world_transform.y.z, main_cam_world_transform.z.z,
        );

        let gizmo_cam_distance = 2.0;
        let gizmo_cam_pos_local = vec3(20.0, 0.0, gizmo_cam_distance);
        let gizmo_cam_pos_world = main_cam_rotation_only_transform.invert().unwrap() * gizmo_cam_pos_local;

        self.gizmo.gizmo_camera.set_view(
            gizmo_cam_pos_world,
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );

        screen
            .clear(ClearState::depth(1.0))
            .render(
                &self.gizmo.gizmo_camera,
                &self.gizmo.gizmo_axes,
                &[&self.light],
            );
/// Represents a small coordinate gizmo for orientation.
pub struct Gizmo {
    pub gizmo_camera: Camera,
    pub gizmo_axes: Axes,
    pub gizmo_size_pixels: f32,
    pub gizmo_padding_pixels: f32,
}

/// Creates and returns a `Gizmo` for the corner of the screen.
pub fn create_gizmo(context: &Context, sim_box_max: f32) -> Gizmo {
    let gizmo_size_pixels: f32 = 200.0;
    let gizmo_height_units = 20.0; // Defines the vertical extent of the orthographic view

    // Note: gizmo_camera initially gets a dummy viewport. It's updated in update_and_render_frame.
    let gizmo_camera = Camera::new_orthographic(
        Viewport { x: 0, y: 0, width: gizmo_size_pixels as u32, height: gizmo_size_pixels as u32 },
        vec3(0.0, 0.0, 5.0), // Eye: fixed distance on Z axis
        vec3(0.0, 0.0, 0.0), // Target: origin of the axes
        vec3(0.0, 1.0, 0.0), // Up: standard Y-up
        gizmo_height_units,  // height
        0.1,                 // z_near
        1000.0,               // z_far
    );

    let gizmo_axes = create_axes(context, sim_box_max);

    Gizmo {
        gizmo_camera,
        gizmo_axes,
        gizmo_size_pixels,
        gizmo_padding_pixels: 0.0,
    }

            */
