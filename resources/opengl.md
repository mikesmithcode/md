# Opengl and three-d

Notes on https://learnopengl.com with any links to three-d (Rust) added. All the examples shown are also translated to Rust open-gl here: https://github.com/srcres258/learnopengl-rust

### Terminology

#### Opengl is a state machine:

**States:**
Represent different conditions or modes the system can be in. For example, in a vending machine, states could be "Idle," "HasCoin," "Dispensing," and "Out of Order". In Opengl the state is called the **context**.

**Transitions:**
Define how the system moves from one state to another based on specific inputs or conditions. For example, in a vending machine, inserting a coin might trigger a transition from "Idle" to "HasCoin". 

**Actions:**
Describe what happens when a transition occurs or while the system is in a particular state. For example, in a vending machine, displaying the price of an item could be an action performed during the "HasCoin" state. 

In Opengl an **Object** is a subset of the **Context**. 

#### Viewport

This describes the location and the size of the window

#### Render Loop

An infinite while loop that runs until we close the window. It polls the context or state for any changes and renders output to the screen.

### Graphics Pipeline

The graphics pipeline handles turning 3D coordinates into 2D pixels.This requires a series of steps that take an input and generate an output. The cores on the gpu run little programs known as Shaders. We'll use a triangle as example.

A single 3D coordinate is a **vertex**. An array of coords is **Vertex Data** which is the input to the graphics pipeline. Each vertex is a bit of data that might for example be the 3D coords and a colour value. 

**A primitive** tells OpenGl about the shape being drawn: lines, points, triangles which is used by drawing commands. 

Vertex shader takes primitive vertices and passes it to Geometry shader and can generate additional primitives by creating new vertices. The shapes are then assembled. The primitives are then rasterized which maps coords to a pixel in 2D. These pixels are fragments that can be coloured. A fragment in OpenGL is all the data required for OpenGL to render a single pixel. Everything outside field of view is discarded. Finally, tests work out how multiple things mapped to same pixel should be handled. Are they hidden? Are they transparent to some degree and therefore need to be mixed with pixels behind.

### Textures

Textures are effectively a picture that is scaled and mapped onto a set of vertexes to prevent you having to specify lots of vertices in between.

### Transformations

Transformations are matrix operations on vectors

### Coordinate systems

1. Local space - Coordinates local to an object

2. World space - Coordinates that you use to position objects within the "world"

3. View space or camera - space as seen by your camera

4. Clip space - Defines which coordinates are visible or not and hence whether they are rendered or not

Coordinates are written (x,y,z,w)

#### Projections

1. Orthographic - Directly maps coords to 2d coords

![alt text](imgs/image-1.png)

2. Perspective - Things further away look smaller. Once the coordinates are transformed to clip space they are in the range -w to w (anything outside this range is clipped).

![alt text](imgs/image.png)

#### axes

Opengl is a RH coord system, where z is out of the page.

![alt text](imgs/image-2.png)

### Camera

Camera needs a bunch of info:

1. Position (3D coords specifying where in world space it is situated)

2. Direction This points from the camera to some point (often the origing) and defines the camera z axis

3. Right axis a vector that is the positive x for the camera

4. Up axis is the camera's y axis

The camera can be moved.
