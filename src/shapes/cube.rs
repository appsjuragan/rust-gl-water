//! Cube shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams, ShapeVertex};
use glam::Vec3;

pub struct Cube;

impl Cube {
    pub fn new() -> Self { Self }
    
    const SIZE_FACTOR: f32 = 0.577;
}

impl Default for Cube {
    fn default() -> Self { Self::new() }
}

impl Shape for Cube {
    fn name(&self) -> &str { "Cube" }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        let half = params.radius * Self::SIZE_FACTOR;
        let q = point.abs() - Vec3::splat(half);
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>) {
        let s = params.radius * Self::SIZE_FACTOR;

        #[rustfmt::skip]
        let vertices = vec![
            ShapeVertex { position: [-s, -s,  s], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [ s, -s,  s], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [ s,  s,  s], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [-s,  s,  s], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
            ShapeVertex { position: [ s, -s, -s], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [-s, -s, -s], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [-s,  s, -s], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [ s,  s, -s], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
            ShapeVertex { position: [ s, -s,  s], normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [ s, -s, -s], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [ s,  s, -s], normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [ s,  s,  s], normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            ShapeVertex { position: [-s, -s, -s], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [-s, -s,  s], normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [-s,  s,  s], normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [-s,  s, -s], normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            ShapeVertex { position: [-s,  s,  s], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [ s,  s,  s], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [ s,  s, -s], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [-s,  s, -s], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
            ShapeVertex { position: [-s, -s, -s], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0] },
            ShapeVertex { position: [ s, -s, -s], normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0] },
            ShapeVertex { position: [ s, -s,  s], normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0] },
            ShapeVertex { position: [-s, -s,  s], normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0] },
        ];

        #[rustfmt::skip]
        let indices = vec![
            0,  1,  2,  0,  2,  3,
            4,  5,  6,  4,  6,  7,
            8,  9,  10, 8,  10, 11,
            12, 13, 14, 12, 14, 15,
            16, 17, 18, 16, 18, 19,
            20, 21, 22, 20, 22, 23,
        ];

        (vertices, indices)
    }

    fn bounding_sphere(&self, params: &ShapeParams) -> (Vec3, f32) {
        (Vec3::ZERO, params.radius)
    }

    fn shader_intersection_code(&self) -> String {
        r#"
fn intersect_cube(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, rotation: vec4<f32>) -> f32 {
    let half_size = radius * 0.577;
    let cube_min = center - vec3<f32>(half_size);
    let cube_max = center + vec3<f32>(half_size);
    let t_min = (cube_min - origin) / ray;
    let t_max = (cube_max - origin) / ray;
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);
    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);
    if (t_near > t_far || t_far < 0.0) { return -1.0; }
    return max(t_near, 0.0);
}
"#.to_string()
    }

    fn shader_sdf_code(&self) -> String {
        r#"
fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}
"#.to_string()
    }

    fn volume(&self, params: &ShapeParams) -> f32 {
        let size = params.radius * Self::SIZE_FACTOR * 2.0;
        size * size * size
    }

    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32 {
        let half = params.radius * Self::SIZE_FACTOR;
        let bottom = center_y - half;
        let top = center_y + half;

        if water_height <= bottom {
            0.0
        } else if water_height >= top {
            self.volume(params)
        } else {
            let full_size = half * 2.0;
            full_size * full_size * (water_height - bottom)
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}
