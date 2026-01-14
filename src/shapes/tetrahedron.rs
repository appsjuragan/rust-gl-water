//! Tetrahedron shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams, ShapeVertex};
use glam::Vec3;

/// Tetrahedron shape
pub struct Tetrahedron;

impl Tetrahedron {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Tetrahedron {
    fn default() -> Self {
        Self::new()
    }
}

impl Shape for Tetrahedron {
    fn name(&self) -> &str {
        "Tetrahedron"
    }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        let p = point / params.radius;
        let dist = ((p.x + p.y).abs() - p.z).max((p.x - p.y).abs() + p.z);
        (dist - 1.0) / 3.0f32.sqrt() * params.radius
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>) {
        let size = params.radius * 1.0;
        let s = size / 3.0f32.sqrt();

        let v0 = Vec3::new(s, s, s);
        let v1 = Vec3::new(s, -s, -s);
        let v2 = Vec3::new(-s, s, -s);
        let v3 = Vec3::new(-s, -s, s);

        let vertices = vec![
            // Face 0-1-2
            ShapeVertex { position: v0.to_array(), normal: Self::face_normal(v0, v1, v2).to_array(), uv: [0.0, 0.0] },
            ShapeVertex { position: v1.to_array(), normal: Self::face_normal(v0, v1, v2).to_array(), uv: [1.0, 0.0] },
            ShapeVertex { position: v2.to_array(), normal: Self::face_normal(v0, v1, v2).to_array(), uv: [0.5, 1.0] },
            // Face 0-2-3
            ShapeVertex { position: v0.to_array(), normal: Self::face_normal(v0, v2, v3).to_array(), uv: [0.0, 0.0] },
            ShapeVertex { position: v2.to_array(), normal: Self::face_normal(v0, v2, v3).to_array(), uv: [1.0, 0.0] },
            ShapeVertex { position: v3.to_array(), normal: Self::face_normal(v0, v2, v3).to_array(), uv: [0.5, 1.0] },
            // Face 0-3-1
            ShapeVertex { position: v0.to_array(), normal: Self::face_normal(v0, v3, v1).to_array(), uv: [0.0, 0.0] },
            ShapeVertex { position: v3.to_array(), normal: Self::face_normal(v0, v3, v1).to_array(), uv: [1.0, 0.0] },
            ShapeVertex { position: v1.to_array(), normal: Self::face_normal(v0, v3, v1).to_array(), uv: [0.5, 1.0] },
            // Face 1-3-2
            ShapeVertex { position: v1.to_array(), normal: Self::face_normal(v1, v3, v2).to_array(), uv: [0.0, 0.0] },
            ShapeVertex { position: v3.to_array(), normal: Self::face_normal(v1, v3, v2).to_array(), uv: [1.0, 0.0] },
            ShapeVertex { position: v2.to_array(), normal: Self::face_normal(v1, v3, v2).to_array(), uv: [0.5, 1.0] },
        ];

        let indices = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

        (vertices, indices)
    }

    fn bounding_sphere(&self, params: &ShapeParams) -> (Vec3, f32) {
        (Vec3::ZERO, params.radius)
    }

    fn shader_intersection_code(&self) -> String {
        r#"
fn intersect_tetrahedron(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, rotation: vec4<f32>) -> f32 {
    let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let local_origin = rotate_vector(origin - center, inv_rotation);
    let local_ray = rotate_vector(ray, inv_rotation);
    
    let p = local_origin / radius;
    let dist = max(abs(p.x + p.y) - p.z, abs(p.x - p.y) + p.z);
    if ((dist - 1.0) / sqrt(3.0) * radius < 0.0) {
        return 0.0;
    }
    
    var t = 0.0;
    for (var i=0; i<64; i++) {
        let p = local_origin + local_ray * t;
        let p_norm = p / radius;
        let d = max(abs(p_norm.x + p_norm.y) - p_norm.z, abs(p_norm.x - p_norm.y) + p_norm.z);
        let dist = (d - 1.0) / sqrt(3.0) * radius;
        if (dist < 0.0005) { return t; }
        t += abs(dist);
        if (t > radius * 4.0) { return -1.0; }
    }
    return -1.0;
}
"#.to_string()
    }

    fn shader_sdf_code(&self) -> String {
        r#"
fn sdTetrahedron(p: vec3<f32>, r: f32) -> f32 {
    let p_norm = p / r;
    let dist = max(abs(p_norm.x + p_norm.y) - p_norm.z, abs(p_norm.x - p_norm.y) + p_norm.z);
    return (dist - 1.0) / sqrt(3.0) * r;
}
"#.to_string()
    }

    fn volume(&self, params: &ShapeParams) -> f32 {
        let a = params.radius * 2.0 / 3.0f32.sqrt();
        a * a * a / (6.0 * 2.0f32.sqrt())
    }

    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32 {
        let depth = water_height - center_y;
        let r = params.radius;

        if depth <= -r {
            0.0
        } else if depth >= r {
            self.volume(params)
        } else {
            let fraction = (depth + r) / (2.0 * r);
            self.volume(params) * fraction.clamp(0.0, 1.0)
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}

impl Tetrahedron {
    fn face_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
        (v1 - v0).cross(v2 - v0).normalize()
    }
}
