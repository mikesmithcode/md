
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Visibility{
    Hidden,
    Transparent,
    Opaque
}


#[derive(Debug, Clone)]
pub enum ObjectSpec{
    WireBox(BoxSpec),
    Rectangle(RectSpec),
    Triangle(TriSpec),
}

impl ObjectSpec{
    ///Returns a reference to the underlying spec e.g BoxSpec
    pub fn get_box_spec(&self) -> Option<BoxSpec> {
        match self {
            ObjectSpec::WireBox(boxspec) => Some(*boxspec),
            _ => None,
        }
    }

    pub fn get_rect_spec(&self) -> Option<RectSpec> {
        match self {
            ObjectSpec::Rectangle(rectspec)=> Some(*rectspec),
            _ => None,
        }
    }

    pub fn get_tri_spec(&self) -> Option<TriSpec> {
        match self {
            ObjectSpec::Triangle(trispec)=> Some(*trispec),
            _ => None,
        }
    }
}


///------------------------------------------------------------------------------
/// BoxSpec
/// 
/// This is the configuration of a box on the simulation side. It is rendered
/// in md_viz by a BoxRenderable in md_viz::objects.rs
///------------------------------------------------------------------------------
/// Configuration for a generic box-like object in the scene.
/// 
/// Fields:
/// 
/// visible - turn graphic display on and off
/// thickness - is internal if negative but external if positive
/// position - this coord sets the centre of the box. The axis of system is 0,0,0 in bottom, left, back corner
/// box_size - dimensions. The axis of system is x across, y front-back, z up down 
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BoxSpec {
    pub id: usize,
    pub thickness: f64, 
    pub center: DVec3,          
    #[serde(skip)]
    pub box_size: DVec3,
    #[serde(skip)]
    pub orientation: DQuat,
    #[serde(skip)]
    pub color: Srgba,
    pub visibility: Visibility
}

impl Default for BoxSpec {
    fn default() -> Self {
        Self {
            id: 0,
            thickness: 0.1,
            center: DVec3::ZERO,
            box_size: DVec3::new(10.0, 0.1, 10.0),
            orientation: DQuat::IDENTITY,
            color: Srgba::WHITE,
            visibility: Visibility::Opaque
        }
    }
}

impl BoxSpec {
    /// Creates a BoxSpec using explicit dimensions, automatically assigning a unique ID.
    pub fn new(center: DVec3, box_size: DVec3, thickness: f64, color: Srgba, visibility: Visibility) -> Self {
        let id = next_id();

        let box_spec = Self {
            id,
            thickness,
            center,
            box_size,
            orientation: DQuat::IDENTITY,
            color,
            visibility
        };
        box_spec.validate();
        box_spec
    }

    /// Applies a rigid-body translation and rotation to the box.
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
    pub id: usize,
    pub center: DVec3,
    pub normal: DVec3,
    pub tangent: DVec3,
    pub velocity: DVec3,
    pub half_size: DVec2,        // [half_width, half_height]
    pub vertices: [DVec3; 4],    // [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
    pub color: Srgba,
    pub visibility: Visibility
}

impl RectSpec {
    /// Creates a RectSpec from 4 corner vertices.
    /// Order expected: [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
    pub fn new(vertices: [DVec3; 4], color: Srgba, visibility: Visibility) -> Self {
        let id = next_id();

        let [v0, v1, v2, v3] = vertices;

        // 1. Calculate center as the average of all 4 corners
        let center = (v0 + v1 + v2 + v3) / 4.0;

        // 2. Calculate edges to find tangent (width direction) and bitangent (height direction)
        let edge_width = v1 - v0;   // Top edge vector
        let edge_height = v2 - v1;  // Right edge vector

        // 3. Half sizes are half the lengths of the respective edges
        let half_width = edge_width.length() * 0.5;
        let half_height = edge_height.length() * 0.5;
        let half_size = DVec2::new(half_width, half_height);

        // 4. Tangent and Normal
        let tangent = edge_width.normalize();
        let normal = edge_width.cross(edge_height).normalize();

        // 5. Convert absolute vertices into local-space coordinates relative to (0,0,0)
        let local_vertices = [
            v0 - center,
            v1 - center,
            v2 - center,
            v3 - center,
        ];

        let velocity = DVec3::ZERO;

        let mut rect = Self {
            id,
            center,
            normal,
            tangent,
            velocity,
            half_size,
            vertices: local_vertices,
            color,
            visibility
        };

        rect.validate();
        rect.update_vertices(); // Populates world-space vertices correctly
        rect
    }

    /// Recalculates vertices based on current center, normal, tangent, and half_size.
    pub fn update_vertices(&mut self) {
        let normal = self.normal.normalize();
        let tangent = (self.tangent - normal * self.tangent.dot(normal)).normalize();
        let bitangent = normal.cross(tangent);

        let hx = self.half_size.x;
        let hy = self.half_size.y;

        let scaled_tangent = tangent * hx;
        let scaled_bitangent = bitangent * hy;

        self.vertices = [
            self.center - scaled_tangent + scaled_bitangent, // Top-Left
            self.center + scaled_tangent + scaled_bitangent, // Top-Right
            self.center + scaled_tangent - scaled_bitangent, // Bottom-Right
            self.center - scaled_tangent - scaled_bitangent, // Bottom-Left
        ];
    }

    /// Applies a rigid-body translation and rotation (via a DQuat) to the plane.
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        self.center += translation_delta;
        if let Some(rot) = rotation {
            self.normal = rot * self.normal;
            self.tangent = rot * self.tangent;
        }
        self.update_vertices();
    }

    pub fn step(&mut self, vel: DVec3, dt: f64){
        self.velocity = vel;
        self.transform(vel*dt, None);
    }

    /// Panics if the rectangle geometry, planarity, or basis vectors are invalid.
    pub fn validate(&self) {
        let n = self.normal.normalize();
        let t = (self.tangent - n * self.tangent.dot(n)).normalize();
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
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct TriSpec {
    pub id: usize,
    pub center: DVec3,
    pub normal: DVec3,          // Primary orientation vector
    pub tangent: DVec3,         // Secondary vector to resolve rotation around the normal
    pub velocity: DVec3,
    pub vertices: [DVec3; 3],    // World-space vertices [v0, v1, v2]
    pub local_triangles: [DVec3; 3], // Pre-scaled raw triangles loaded relative to (0,0,0)
    pub color: Srgba,
    pub visibility: Visibility 
}

impl TriSpec {
    /// Creates a TriSpec from 3 corner vertices.
    pub fn new(vertices: [DVec3; 3], color: Srgba, visibility: Visibility) -> Self {
        let id = next_id();
        
        let [v0, v1, v2] = vertices;

        assert!(
            !v0.is_nan() && !v1.is_nan() && !v2.is_nan(),
            "TriSpec (id: {}) error: One or more input vertices contain NaN values.",
            id
        );

        // 2. Calculate edges
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        let cross = edge1.cross(edge2);
        assert!(
            cross.length_squared() > 1.0e-14,
            "TriSpec (id: {}) is degenerate (zero area or duplicate vertices). Vertices: {:?}",
            id, vertices
        );

        // 3. Calculate center as the average of the 3 vertices
        let center = (v0 + v1 + v2) / 3.0;

        // 4. Normal from cross product (STL right-hand rule convention)
        let normal = cross.normalize();

        // 5. Tangent along the first edge direction
        let tangent = edge1.normalize();

        // 6. Convert absolute vertices into local-space coordinates relative to (0,0,0)
        let local_triangles = [
            v0 - center,
            v1 - center,
            v2 - center,
        ];

        let velocity = DVec3::ZERO;

        let tri = Self {
            id,
            center,
            normal,
            tangent,
            velocity,
            vertices,
            local_triangles,
            color,
            visibility
        };

        tri.validate();
        tri
    }

    /// Recalculates world-space vertices based on current center, normal, tangent, and local geometry.
    pub fn update_vertices(&mut self) {
        let normal = self.normal.normalize();
        let tangent = (self.tangent - normal * self.tangent.dot(normal)).normalize();
        let bitangent = normal.cross(tangent);

        self.vertices = [
            self.transform_point(self.local_triangles[0], &tangent, &bitangent, &normal),
            self.transform_point(self.local_triangles[1], &tangent, &bitangent, &normal),
            self.transform_point(self.local_triangles[2], &tangent, &bitangent, &normal),
        ];
    }

    #[inline]
    fn transform_point(&self, p: DVec3, t: &DVec3, b: &DVec3, n: &DVec3) -> DVec3 {
        let rotated = *t * p.x + *b * p.y + *n * p.z;
        rotated + self.center
    }

    /// Applies a rigid-body translation and rotation (via a DQuat) to the triangle.
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        // Update center
        self.center += translation_delta;

        // If a rotation is provided, rotate the normal and tangent vectors
        if let Some(rot) = rotation {
            self.normal = rot * self.normal;
            self.tangent = rot * self.tangent;
        }

        // Recompute vertices and normalize basis vectors together
        self.update_vertices();
    }


    pub fn step(&mut self, vel: DVec3, dt: f64){
        self.velocity = vel;
        self.transform(vel*dt, None);
    }

    /// Directly set a new position and orientation.
    pub fn set(&mut self, new_center: DVec3, new_normal: DVec3, new_tangent: DVec3) {
        self.center = new_center;
        self.normal = new_normal;
        self.tangent = new_tangent;
        self.update_vertices();
    }

    /// Panics if normal and tangent are not orthogonal.
    pub fn validate(&self) {
        let n = self.normal.normalize();
        let t = (self.tangent - n * self.tangent.dot(n)).normalize();
        
        assert!(
            n.dot(t).abs() < 1e-5,
            "TriSpec (id: {}) normal and tangent are not orthogonal!",
            self.id
        );
    }
}