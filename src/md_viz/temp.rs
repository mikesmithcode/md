use crate::renderer::*;

///
/// A bounding box geometry used for visualising an [AxisAlignedBoundingBox].
///
pub struct BoxMesh {
    mesh: InstancedMesh,
}

impl BoxMesh {
    ///
    /// Creates a skeleton-style bounding box geometry from an axis aligned bounding box.
    ///
    pub fn new(context: &Context, aabb: AxisAlignedBoundingBox) -> Self {
        let size = aabb.size();
        let thickness = 0.02 * size.x.max(size.y).max(size.z);

        Self::new_skeleton_box(context, aabb, thickness)
    }

    ///
    /// Creates a skeleton-style bounding box with a specific line thickness.
    /// Positive thickness expands the shell outward, negative thickness expands it inward.
    ///
    pub fn new_skeleton_box(
        context: &Context,
        aabb: AxisAlignedBoundingBox,
        thickness: f32,
    ) -> Self {
        let max = aabb.max();
        let min = aabb.min();
        let thickness = thickness.abs();
        let offset = if thickness <= 0.0 { 0.0 } else { thickness };
        let effective_min = if thickness <= 0.0 {
            min
        } else {
            vec3(min.x - offset, min.y - offset, min.z - offset)
        };
        let effective_max = if thickness <= 0.0 {
            max
        } else {
            vec3(max.x + offset, max.y + offset, max.z + offset)
        };
        let size = effective_max - effective_min;
        let translations = vec![
            vec3(effective_min.x, effective_min.y, effective_min.z),
            vec3(effective_min.x, effective_max.y, effective_max.z),
            vec3(effective_min.x, effective_min.y, effective_max.z),
            vec3(effective_min.x, effective_max.y, effective_min.z),
            vec3(effective_min.x, effective_min.y, effective_min.z),
            vec3(effective_max.x, effective_min.y, effective_max.z),
            vec3(effective_min.x, effective_min.y, effective_max.z),
            vec3(effective_max.x, effective_min.y, effective_min.z),
            vec3(effective_min.x, effective_min.y, effective_min.z),
            vec3(effective_max.x, effective_max.y, effective_min.z),
            vec3(effective_min.x, effective_max.y, effective_min.z),
            vec3(effective_max.x, effective_min.y, effective_min.z),
        ];

        let mesh = InstancedMesh::new(
            context,
            &Instances {
                transformations: (0..12)
                    .map(|i| {
                        let midpoint = 0.5 * (translations[i] + match i {
                            0 | 1 | 2 | 3 => vec3(effective_max.x, effective_min.y, effective_min.z),
                            4 | 5 | 6 | 7 => vec3(effective_min.x, effective_max.y, effective_min.z),
                            8 | 9 | 10 | 11 => vec3(effective_min.x, effective_min.y, effective_max.z),
                            _ => unreachable!(),
                        });
                        Mat4::from_translation(midpoint)
                            * match i {
                                0..=3 => Mat4::from_nonuniform_scale(size.x, thickness, thickness),
                                4..=7 => {
                                    Mat4::from_nonuniform_scale(thickness, size.y, thickness)
                                }
                                8..=11 => {
                                    Mat4::from_nonuniform_scale(thickness, thickness, size.z)
                                }
                                _ => unreachable!(),
                            }
                    })
                    .collect(),
                ..Default::default()
            },
            &CpuMesh::cube(),
        );
        Self { mesh }
    }

    ///
    /// Creates a hollow box with filled faces from an axis aligned bounding box.
    /// Positive thickness expands the shell outward, negative thickness expands it inward.
    ///
    pub fn new_box(context: &Context, aabb: AxisAlignedBoundingBox, thickness: f32) -> Self {
        let min = aabb.min();
        let max = aabb.max();
        let thickness = thickness.abs();
        let (outer_min, outer_max, inner_min, inner_max) = if thickness <= 0.0 {
            (min, max, min, max)
        } else if thickness > 0.0 {
            (
                vec3(min.x - thickness, min.y - thickness, min.z - thickness),
                vec3(max.x + thickness, max.y + thickness, max.z + thickness),
                min,
                max,
            )
        } else {
            (
                min,
                max,
                vec3(min.x + thickness.abs(), min.y + thickness.abs(), min.z + thickness.abs()),
                vec3(max.x - thickness.abs(), max.y - thickness.abs(), max.z - thickness.abs()),
            )
        };
        let transformations = vec![
            (
                vec3((outer_min.x + inner_min.x) * 0.5, (outer_min.y + outer_max.y) * 0.5, (outer_min.z + outer_max.z) * 0.5),
                vec3(thickness, outer_max.y - outer_min.y, outer_max.z - outer_min.z),
            ),
            (
                vec3((outer_max.x + inner_max.x) * 0.5, (outer_min.y + outer_max.y) * 0.5, (outer_min.z + outer_max.z) * 0.5),
                vec3(thickness, outer_max.y - outer_min.y, outer_max.z - outer_min.z),
            ),
            (
                vec3((outer_min.x + outer_max.x) * 0.5, (outer_min.y + inner_min.y) * 0.5, (outer_min.z + outer_max.z) * 0.5),
                vec3(outer_max.x - outer_min.x, thickness, outer_max.z - outer_min.z),
            ),
            (
                vec3((outer_min.x + outer_max.x) * 0.5, (outer_max.y + inner_max.y) * 0.5, (outer_min.z + outer_max.z) * 0.5),
                vec3(outer_max.x - outer_min.x, thickness, outer_max.z - outer_min.z),
            ),
            (
                vec3((outer_min.x + outer_max.x) * 0.5, (outer_min.y + outer_max.y) * 0.5, (outer_min.z + inner_min.z) * 0.5),
                vec3(outer_max.x - outer_min.x, outer_max.y - outer_min.y, thickness),
            ),
            (
                vec3((outer_min.x + outer_max.x) * 0.5, (outer_min.y + outer_max.y) * 0.5, (outer_max.z + inner_max.z) * 0.5),
                vec3(outer_max.x - outer_min.x, outer_max.y - outer_min.y, thickness),
            ),
        ];

        let mesh = InstancedMesh::new(
            context,
            &Instances {
                transformations: transformations
                    .into_iter()
                    .map(|(translation, scale)| {
                        Mat4::from_translation(translation)
                            * Mat4::from_nonuniform_scale(scale.x, scale.y, scale.z)
                    })
                    .collect(),
                ..Default::default()
            },
            &CpuMesh::cube(),
        );
        Self { mesh }
    }
}

impl<'a> IntoIterator for &'a BoxMesh {
    type Item = &'a dyn Geometry;
    type IntoIter = std::iter::Once<&'a dyn Geometry>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

use std::ops::Deref;
impl Deref for BoxMesh {
    type Target = InstancedMesh;
    fn deref(&self) -> &Self::Target {
        &self.mesh
    }
}

impl std::ops::DerefMut for BoxMesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mesh
    }
}

impl Geometry for BoxMesh {
    impl_geometry_body!(deref);

    fn animate(&mut self, time: f32) {
        self.mesh.animate(time)
    }
}
