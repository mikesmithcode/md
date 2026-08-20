
use three_d::Srgba;
use serde::{Serialize, Deserialize};
use glam::{DVec3, DVec2, DQuat};
use std::sync::atomic::{AtomicUsize, Ordering};

// Global atomic counter starting at 0
static GLOBAL_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generates the next unique ID across all spec types.
pub fn next_id() -> usize {
    GLOBAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub enum ObjectSpec {
    /// A wireframe or solid box object specification.
    WireBox(BoxSpec),
    /// A 2D rectangular plane object specification.
    Rectangle(RectSpec),
    /// A triangular surface object specification.
    Triangle(TriSpec),
}

impl ObjectSpec {
    /// Returns an optional reference to the underlying `BoxSpec` if this variant is a wire box.
    pub fn get_box_spec(&self) -> Option<BoxSpec> {
        match self {
            ObjectSpec::WireBox(boxspec) => Some(*boxspec),
            _ => None,
        }
    }

    /// Returns an optional reference to the underlying `RectSpec` if this variant is a rectangle.
    pub fn get_rect_spec(&self) -> Option<RectSpec> {
        match self {
            ObjectSpec::Rectangle(rectspec) => Some(*rectspec),
            _ => None,
        }
    }

    /// Returns an optional reference to the underlying `TriSpec` if this variant is a triangle.
    pub fn get_tri_spec(&self) -> Option<TriSpec> {
        match self {
            ObjectSpec::Triangle(trispec) => Some(*trispec),
            _ => None,
        }
    }
}


///------------------------------------------------------------------------------
/// BoxSpec
/// 
/// This is the configuration of a box on the simulation side. It is rendered
/// in md_viz by a WireBoxTemplate in md_viz::templates.rs
///------------------------------------------------------------------------------
/// Configuration for a generic wire box-like object in the scene. Used to visualise the extent of the simulation box.
/// 
/// Fields:
/// 
/// visible - turn display of item on and off
/// thickness - is internal if negative but external if positive
/// position - this coord sets the centre of the box. The axis of system is 0,0,0 in bottom, left, back corner
/// box_size - dimensions. The axis of system is x across, y front-back, z up down 
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BoxSpec {
    /// Unique identifier for the box instance.
    pub id: usize,
    /// Collision or rendering boundary thickness (internal if negative, external if positive).
    pub thickness: f64, 
    /// Center coordinates of the box in world space.
    pub center: DVec3,          
    #[serde(skip)]
    /// Dimensions of the box along the x, y, and z axes.
    pub box_size: DVec3,
    #[serde(skip)]
    /// Orientation quaternion representing rotation from local to world space.
    pub orientation: DQuat,
    #[serde(skip)]
    /// RGBA colour representation for rendering.
    pub colour: Srgba,
    /// Flag determining whether the box is rendered in the visualization scene.
    pub visible: bool
}

impl Default for BoxSpec {
    /// Returns default configuration values for a standard simulation boundary box.
    fn default() -> Self {
        Self {
            id: 0,
            thickness: 0.1,
            center: DVec3::ZERO,
            box_size: DVec3::new(10.0, 0.1, 10.0),
            orientation: DQuat::IDENTITY,
            colour: Srgba::WHITE,
            visible: true
        }
    }
}

impl BoxSpec {
    /// Creates a BoxSpec using explicit dimensions and attributes, automatically assigning a unique object ID.
    ///
    /// # Arguments
    ///
    /// * `center` - Center position vector in world space.
    /// * `box_size` - Full dimensions along the x, y, and z axes.
    /// * `thickness` - Shell thickness value.
    /// * `colour` - RGBA visual colour.
    /// * `visible` - Display toggle flag.
    ///
    /// # Returns
    ///
    /// * `Self` - An validated instance of `BoxSpec`.
    pub fn new(center: DVec3, box_size: DVec3, thickness: f64, colour: Srgba, visible: bool) -> Self {
        let id = next_id();

        let box_spec = Self {
            id,
            thickness,
            center,
            box_size,
            orientation: DQuat::IDENTITY,
            colour,
            visible
        };
        box_spec.validate();
        box_spec
    }

    /// Applies a rigid-body translation delta and optional rotation to the box.
    ///
    /// # Arguments
    ///
    /// * `translation_delta` - Vector displacement added to the center.
    /// * `rotation` - Optional rotational quaternion to multiply against the current orientation.
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        self.center += translation_delta;
        if let Some(rot) = rotation {
            self.orientation = rot * self.orientation;
        }
    }

    /// Panics if the box dimensions are non-positive.
    pub fn validate(&self) {
        assert!(
            self.box_size.x > 0.0 && self.box_size.y > 0.0 && self.box_size.z > 0.0,
            "BoxSpec (id: {}) error: box_size dimensions must be positive, got {:?}",
            self.id,
            self.box_size
        );
    }
}


/// 2D rectangular plane in 3d space
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectSpec {
    /// Unique identifier for the rectangle instance.
    pub id: usize,
    /// Center position vector in world space.
    pub center: DVec3,
    /// Linear velocity vector of the rectangle.
    pub velocity: DVec3,
    /// Rotation quaternion transforming local space to world space.
    pub orientation: DQuat,
    /// Angular velocity vector.
    pub omega: DVec3,
    /// Half-dimensions along the local axes `[half_width, half_height]`.
    pub half_size: DVec2,
    /// Evaluated world-space coordinates of the four corners.
    pub vertices: [DVec3; 4],
    /// RGBA colour representation for rendering.
    pub colour: Srgba,
    /// Flag determining whether the rectangle is rendered in the visualization scene.
    pub visible: bool,
}

impl RectSpec {
    /// Creates a RectSpec from 4 corner vertices.
    /// Order expected: [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of four world-space vertex positions.
    /// * `colour` - RGBA visual colour.
    /// * `visible` - Display toggle.
    ///
    /// # Returns
    ///
    /// * `Self` - An initialized and validated `RectSpec` instance.
    pub fn new(vertices: [DVec3; 4], colour: Srgba, visible: bool) -> Self {
        let id = next_id();

        let [v0, v1, v2, v3] = vertices;

        // 1. Calculate center as the average of all 4 corners
        let center = (v0 + v1 + v2 + v3) / 4.0;

        // 2. Calculate edges to find tangent (width direction) and bitangent (height direction)
        let edge_width = v1 - v0;   // Top edge vector
        let edge_height = v3 - v0;  // Left edge vector (using v3 - v0 matches Top-Left to Bottom-Left)

        // 3. Half sizes are half the lengths of the respective edges
        let half_width = edge_width.length() * 0.5;
        let half_height = edge_height.length() * 0.5;
        let half_size = DVec2::new(half_width, half_height);

        // 4. Build local basis vectors
        let tangent = edge_width.normalize();
        let normal = edge_width.cross(edge_height).normalize();
        let bitangent = normal.cross(tangent).normalize();

        // 5. Build orientation quaternion from the rotation matrix columns
        let mat3 = glam::DMat3::from_cols(tangent, bitangent, normal);
        let orientation = glam::DQuat::from_mat3(&mat3);

        let mut rect = Self {
            id,
            center,
            orientation,
            velocity: DVec3::ZERO,
            omega: DVec3::ZERO,
            half_size,
            vertices: [DVec3::ZERO; 4],
            colour,
            visible,
        };

        rect.validate();
        rect.update_vertices(); // Populates world-space vertices correctly
        rect
    }

    /// Helper to get the world-space normal on the fly
    pub fn normal(&self) -> DVec3 {
        self.orientation * DVec3::Z
    }

    /// Helper to get the world-space tangent (local X axis)
    pub fn tangent(&self) -> DVec3 {
        self.orientation * DVec3::X
    }

    /// Helper to get the world-space bitangent (local Y axis)
    pub fn bitangent(&self) -> DVec3 {
        self.orientation * DVec3::Y
    }

    /// Recalculates world-space vertices based on current center, orientation, and half_size.
    pub fn update_vertices(&mut self) {
        let hx = self.half_size.x;
        let hy = self.half_size.y;

        // Local corners relative to center: [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
        let local_verts = [
            DVec3::new(-hx,  hy, 0.0), // Top-Left
            DVec3::new( hx,  hy, 0.0), // Top-Right
            DVec3::new( hx, -hy, 0.0), // Bottom-Right
            DVec3::new(-hx, -hy, 0.0), // Bottom-Left
        ];

        // Transform local corners to world space using orientation and center
        self.vertices = [
            self.center + self.orientation * local_verts[0],
            self.center + self.orientation * local_verts[1],
            self.center + self.orientation * local_verts[2],
            self.center + self.orientation * local_verts[3],
        ];
    }

    /// Applies a rigid-body translation and rotation (via a DQuat) to the plane.
    ///
    /// # Arguments
    ///
    /// * `translation_delta` - Positional displacement vector.
    /// * `rotation` - Optional rotation quaternion delta.
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        self.center += translation_delta;
        if let Some(rot) = rotation {
            self.orientation = rot * self.orientation;
        }
        self.update_vertices();
    }

    /// Advances the rectangle's position and orientation over time step `dt`.
    ///
    /// # Arguments
    ///
    /// * `vel` - Linear velocity vector.
    /// * `omega` - Angular velocity vector.
    /// * `dt` - Time step size.
    pub fn step(&mut self, vel: DVec3, omega: DVec3, dt: f64) {
        self.velocity = vel;
        self.omega = omega;

        let translation_delta = vel * dt;
        
        // Convert angular velocity vector (omega * dt) into a rotation quaternion update delta
        let angle = omega.length() * dt;
        let rotation_delta = if angle > 1e-12 {
            Some(glam::DQuat::from_axis_angle(omega.normalize(), angle))
        } else {
            None
        };

        self.transform(translation_delta, rotation_delta);
    }

    /// Panics if the rectangle geometry, planarity, or basis vectors are invalid.
    pub fn validate(&self) {
        let n = self.normal();
        let t = self.tangent();
        let dot = n.dot(t);

        assert!(
            dot.abs() < 1e-4,
            "RectSpec error: Normal and tangent are not orthogonal! Dot product was {}.",
            dot
        );

        assert!(
            self.half_size.x > 0.0 && self.half_size.y > 0.0,
            "RectSpec error: half_size must be positive, got {:?}",
            self.half_size
        );
    }
}



///--------------------------------------------------------------------------------------------------------
/// TriSpec
/// -------------------------------------------------------------------------------------------------------
/// 3D triangular surface in space
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct TriSpec {
    /// Unique identifier for the triangle instance.
    pub id: usize,
    /// Center position vector in world space.
    pub center: DVec3,
    /// Linear velocity vector of the triangle.
    pub velocity: DVec3,
    /// Rotation quaternion transforming local space to world space.
    pub orientation: DQuat,
    /// Angular velocity vector.
    pub omega: DVec3,
    /// Evaluated world-space coordinates of the three corners `[v0, v1, v2]`.
    pub vertices: [DVec3; 3],    
    /// Pre-scaled raw vertices stored relative to the local center `(0,0,0)`.
    pub local_vertices: [DVec3; 3], 
    /// RGBA colour representation for rendering.
    pub colour: Srgba,
    /// Flag determining whether the triangle is rendered in the visualization scene.
    pub visible: bool,
}

impl TriSpec {
    /// Creates a TriSpec from 3 corner vertices.
    ///
    /// # Arguments
    ///
    /// * `vertices` - Array of three world-space vertex positions.
    /// * `colour` - RGBA visual colour.
    /// * `visible` - Display toggle flag.
    ///
    /// # Returns
    ///
    /// * `Self` - An initialized and validated `TriSpec` instance.
    pub fn new(vertices: [DVec3; 3], colour: Srgba, visible: bool) -> Self {
        let id = next_id();
        
        let [v0, v1, v2] = vertices;

        assert!(
            !v0.is_nan() && !v1.is_nan() && !v2.is_nan(),
            "TriSpec (id: {}) error: One or more input vertices contain NaN values.",
            id
        );

        // 1. Calculate edges
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        let cross = edge1.cross(edge2);
        assert!(
            cross.length_squared() > 1.0e-14,
            "TriSpec (id: {}) is degenerate (zero area or duplicate vertices). Vertices: {:?}",
            id, vertices
        );

        // 2. Calculate center as the average of the 3 vertices
        let center = (v0 + v1 + v2) / 3.0;

        // 3. Build local basis vectors
        let tangent = edge1.normalize();
        let normal = cross.normalize();
        let bitangent = normal.cross(tangent).normalize();

        // 4. Build orientation quaternion from the rotation matrix columns
        let mat3 = glam::DMat3::from_cols(tangent, bitangent, normal);
        let orientation = glam::DQuat::from_mat3(&mat3);

        // 5. Convert absolute vertices into local-space coordinates relative to (0,0,0)
        let local_vertices = [
            v0 - center,
            v1 - center,
            v2 - center,
        ];

        let mut tri = Self {
            id,
            center,
            velocity: DVec3::ZERO,
            orientation,
            omega: DVec3::ZERO,
            vertices: [DVec3::ZERO; 3],
            local_vertices,
            colour,
            visible,
        };

        tri.validate();
        tri.update_vertices(); // Populates world-space vertices correctly
        tri
    }

    /// Helper to get the world-space normal on the fly
    pub fn normal(&self) -> DVec3 {
        self.orientation * DVec3::Z
    }

    /// Helper to get the world-space tangent (local X axis)
    pub fn tangent(&self) -> DVec3 {
        self.orientation * DVec3::X
    }

    /// Helper to get the world-space bitangent (local Y axis)
    pub fn bitangent(&self) -> DVec3 {
        self.orientation * DVec3::Y
    }

    /// Recalculates world-space vertices based on current center, orientation, and local geometry.
    pub fn update_vertices(&mut self) {
        self.vertices = [
            self.center + self.orientation * self.local_vertices[0],
            self.center + self.orientation * self.local_vertices[1],
            self.center + self.orientation * self.local_vertices[2],
        ];
    }

    /// Applies a rigid-body translation and rotation (via a DQuat) to the triangle.
    ///
    /// # Arguments
    ///
    /// * `translation_delta` - Positional displacement vector.
    /// * `rotation` - Optional rotation quaternion delta.
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        self.center += translation_delta;
        if let Some(rot) = rotation {
            self.orientation = rot * self.orientation;
        }
        self.update_vertices();
    }

    /// Advances the triangle's position and orientation over time step `dt`.
    ///
    /// # Arguments
    ///
    /// * `vel` - Linear velocity vector.
    /// * `omega` - Angular velocity vector.
    /// * `dt` - Time step size.
    pub fn step(&mut self, vel: DVec3, omega: DVec3, dt: f64) {
        self.velocity = vel;
        self.omega = omega;

        let translation_delta = vel * dt;
        
        // Convert angular velocity vector (omega * dt) into a rotation quaternion update delta
        let angle = omega.length() * dt;
        let rotation_delta = if angle > 1e-12 {
            Some(glam::DQuat::from_axis_angle(omega.normalize(), angle))
        } else {
            None
        };

        self.transform(translation_delta, rotation_delta);
    }

    /// Directly set a new position and orientation.
    pub fn set(&mut self, new_center: DVec3, new_orientation: DQuat) {
        self.center = new_center;
        self.orientation = new_orientation;
        self.update_vertices();
    }

    /// Panics if the triangle geometry or orientation basis vectors are invalid.
    pub fn validate(&self) {
        let n = self.normal();
        let t = self.tangent();
        let dot = n.dot(t);

        assert!(
            dot.abs() < 1e-4,
            "TriSpec error: Normal and tangent are not orthogonal! Dot product was {}.",
            dot
        );

        // Verify that the local vertices are not degenerate
        let [v0, v1, v2] = self.local_vertices;
        let cross = (v1 - v0).cross(v2 - v0);
        assert!(
            cross.length_squared() > 1.0e-14,
            "TriSpec (id: {}) error: Local vertices are degenerate (zero area).",
            self.id
        );
    }
}