## Cameras (md_viz::camera)

The cameras are created using a few parameters which live in the scene_settings.json and the `<simulation_name>.json`

Camera reads the sim_box_size from `<simulation_name>.json` and tries to adjust the field of view to match with a bit of a buffer.
The Camera points at the centre of the simulation box along a vector which is `-rel_pos` (see scene_settings.json). You can also define `up` which defines which axis is the vertical direction of your simulation. **up and rel_pos must be orthogonal** to define a realisable view. If they are not orthogonal you'll get a black screen. 

There are two types of camera:
- Orthographical - this camera has no perspective everything looks the same size regardless of how far from the camera it is. There is no parallax error as a result.
- Perspective - this gives a more natural view where depth changes the apparent size. The `fov` parameter specifies the angle of acceptance in degrees of the camera. if you make this really small it will look more like the orthographic view.

## Lights (md_viz::lights)

Currently, the lights are hard coded. They have a direction and colour.
