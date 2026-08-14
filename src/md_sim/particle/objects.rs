
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
        pub fn new(
        center: DVec3,
        normal: DVec3,
        tangent: DVec3,
        half_size: DVec2,
        color: three_d::Srgba,
    ) -> Self {
        let mut rect = Self {
            center,
            normal,
            tangent,
            half_size,
            color,
            vertices: [DVec3::ZERO; 4], // Temporary placeholder
        };
        rect.update_vertices();
        rect
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
}