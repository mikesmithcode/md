
    use three_d::Srgba;
    use serde::{Serialize, Deserialize};
    use glam::{DVec3, DVec2, DQuat};


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
        pub visible: bool,
        pub thickness: f64, 
        pub position: DVec3,
        #[serde(skip)]
        pub box_size: DVec3,
        #[serde(skip)]
        pub orientation: DQuat,
        #[serde(skip)]
        pub color: Srgba,
    }

    impl Default for BoxSpec {
        fn default() -> Self {
            Self {
                id: 0,
                visible: true,
                thickness: 0.1,
                position: DVec3::ZERO,
                box_size: DVec3::new(10.0, 0.1, 10.0),
                orientation: DQuat::IDENTITY,
                color: Srgba::WHITE,
            }
        }
    }

    impl BoxSpec {
        pub fn new(id: usize, box_size: DVec3, thickness: f64) -> Self {
            Self {
                id,
                box_size,
                thickness,
                ..Default::default()
            }
        }
    }


    /// 2D rectangular plane in 3d space
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct RectSpec {
        pub center: DVec3,
        pub normal: DVec3,
        pub tangent: DVec3,
        pub half_size: DVec2,        // [half_width, half_height]
        pub color: three_d::Srgba,
        pub vertices: [DVec3; 4],    // [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
    }

    impl RectSpec {
        /// Creates a RectSpec from 4 corner vertices.
        /// Order expected: [Top-Left, Top-Right, Bottom-Right, Bottom-Left]
        pub fn new(vertices: [DVec3; 4], color: three_d::Srgba) -> Self {
            let [v0, v1, v2, v3] = vertices;

            // --- 1. Validate input vertices before doing any math ---
            assert!(
                !v0.is_nan() && !v1.is_nan() && !v2.is_nan() && !v3.is_nan(),
                "RectSpec error: One or more input vertices contain NaN values."
            );

            let e01 = v1 - v0; // Top edge
            let e12 = v2 - v1; // Right edge
            let e23 = v3 - v2; // Bottom edge
            let e30 = v0 - v3; // Left edge

            assert!(
                e01.length_squared() > 1.0e-12 && e12.length_squared() > 1.0e-12,
                "RectSpec error: Degenerate rectangle with zero-length edges."
            );

            // Planarity check
            let n1 = e01.cross(e12).normalize();
            let n2 = e23.cross(e30).normalize();
            assert!(
                n1.dot(n2) > 0.999,
                "RectSpec error: The 4 input vertices are not coplanar."
            );

            // Orthogonality check (adjacent edges must be perpendicular)
            let dot_product = e01.normalize().dot(e12.normalize());
            assert!(
                dot_product.abs() < 1e-4,
                "RectSpec error: Adjacent edges are not perpendicular! Dot product was {}.",
                dot_product
            );
            // --------------------------------------------------------

            // 2. Calculate center as the average of all 4 corners
            let center = (v0 + v1 + v2 + v3) / 4.0;

            // 3. Half sizes are half the lengths of the respective edges
            let half_width = e01.length() * 0.5;
            let half_height = e12.length() * 0.5;
            let half_size = DVec2::new(half_width, half_height);

            // 4. Tangent and Normal
            let tangent = e01.normalize();
            let normal = e01.cross(e12).normalize();

            // 5. Convert absolute vertices into local-space coordinates relative to (0,0,0)
            let local_vertices = [
                v0 - center,
                v1 - center,
                v2 - center,
                v3 - center,
            ];

            println!("center {:?}",center);
            println!("half_size {:?}",half_size);

            Self {
                center,
                normal,
                tangent,
                half_size,
                color,
                vertices: local_vertices,
            }
        }


        /// Recalculates vertices based on current center, normal, tangent, and half_size.
        /// tangent points along surface in width direction, bitangent points along surface in height direction.
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
    }



///--------------------------------------------------------------------------------------------------------
/// TriSpec
/// -------------------------------------------------------------------------------------------------------
#[derive(Clone, Debug, Copy, PartialEq)]
pub struct TriSpec {
    pub id: usize,
    pub visible: bool,
    pub center: DVec3,
    pub normal: DVec3,          // Primary orientation vector
    pub tangent: DVec3,         // Secondary vector to resolve rotation around the normal
    pub color: Srgba,
    pub local_triangles: [DVec3; 3], // Pre-scaled raw triangles loaded from STL relative to (0,0,0)
}

impl TriSpec {
    pub fn new(id: usize, v0: DVec3, v1: DVec3, v2: DVec3, color: Srgba) -> Self {
        // 1. Calculate center as the average of the 3 vertices
        let center = (v0 + v1 + v2) / 3.0;

        // 2. Calculate edges
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        let cross = edge1.cross(edge2);
        assert!(
            cross.length_squared() > 1.0e-14,
            "TriSpec (id: {}) is degenerate (zero area or duplicate vertices). Vertices: {:?}, {:?}, {:?}",
            id, v0, v1, v2
        );

        // Normal from cross product (STL right-hand rule convention)
        let normal = edge1.cross(edge2).normalize();

        // Tangent along the first edge direction
        let tangent = edge1.normalize();

        // Convert absolute vertices into local-space coordinates relative to (0,0,0)
        let local_triangles = [
            v0 - center,
            v1 - center,
            v2 - center,
        ];

        let mut tri = Self {
            id,
            visible: true,
            center,
            normal,
            tangent,
            color,
            local_triangles,
        };

        // Validate that everything checks out
        tri.validate();

        tri
    }

    /// Ensures normal and tangent are orthogonal and normalized.
    pub fn normalize_basis(&mut self) {
        self.normal = self.normal.normalize();
        self.tangent = (self.tangent - self.normal * self.tangent.dot(self.normal)).normalize();
    }

    /// Generates the world-space triangles by applying translation and rotation (via normal/tangent).
    pub fn world_triangles(&self) -> [DVec3; 3] {
        let normal = self.normal.normalize();
        let tangent = (self.tangent - normal * self.tangent.dot(normal)).normalize();
        let bitangent = normal.cross(tangent);

        let [v0, v1, v2] = self.local_triangles;

        [
            self.transform_point(v0, &tangent, &bitangent, &normal),
            self.transform_point(v1, &tangent, &bitangent, &normal),
            self.transform_point(v2, &tangent, &bitangent, &normal),
        ]
    }

    #[inline]
    fn transform_point(&self, p: DVec3, t: &DVec3, b: &DVec3, n: &DVec3) -> DVec3 {
        // Rotate local relative point using basis vectors, then translate to center
        let rotated = *t * p.x + *b * p.y + *n * p.z;
        rotated + self.center
    }

    /// Rigid-body transform update
    pub fn transform(&mut self, translation_delta: DVec3, rotation: Option<DQuat>) {
        self.center += translation_delta;
        if let Some(rot) = rotation {
            self.normal = rot * self.normal;
            self.tangent = rot * self.tangent;
        }
        self.normalize_basis();
    }

    /// Directly set a new position and orientation.
    pub fn set(&mut self, new_center: DVec3, new_normal: DVec3, new_tangent: DVec3) {
        self.center = new_center;
        self.normal = new_normal;
        self.tangent = new_tangent;
        self.normalize_basis();
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