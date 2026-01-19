//! Tetrahedron shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams};
use crate::core::geometry::Vertex;
use glam::Vec3;

pub struct Tetrahedron;

impl Tetrahedron {
    pub fn new() -> Self { Self }

    fn face_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
        (v1 - v0).cross(v2 - v0).normalize()
    }
}

impl Default for Tetrahedron {
    fn default() -> Self { Self::new() }
}

impl Shape for Tetrahedron {
    fn name(&self) -> &str { "Tetrahedron" }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        let p = point / params.radius;
        let dist = ((p.x + p.y).abs() - p.z).max((p.x - p.y).abs() + p.z);
        (dist - 1.0) / 3.0f32.sqrt() * params.radius
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<Vertex>, Vec<u32>) {
        let s = params.radius / 3.0f32.sqrt();
        let v0 = Vec3::new(s, s, s);
        let v1 = Vec3::new(s, -s, -s);
        let v2 = Vec3::new(-s, s, -s);
        let v3 = Vec3::new(-s, -s, s);

        let n0 = Self::face_normal(v0, v1, v2);
        let n1 = Self::face_normal(v0, v2, v3);
        let n2 = Self::face_normal(v0, v3, v1);
        let n3 = Self::face_normal(v1, v3, v2);

        let vertices = vec![
            Vertex { position: v0.to_array(), normal: n0.to_array(), uv: [0.0, 0.0] },
            Vertex { position: v1.to_array(), normal: n0.to_array(), uv: [1.0, 0.0] },
            Vertex { position: v2.to_array(), normal: n0.to_array(), uv: [0.5, 1.0] },
            Vertex { position: v0.to_array(), normal: n1.to_array(), uv: [0.0, 0.0] },
            Vertex { position: v2.to_array(), normal: n1.to_array(), uv: [1.0, 0.0] },
            Vertex { position: v3.to_array(), normal: n1.to_array(), uv: [0.5, 1.0] },
            Vertex { position: v0.to_array(), normal: n2.to_array(), uv: [0.0, 0.0] },
            Vertex { position: v3.to_array(), normal: n2.to_array(), uv: [1.0, 0.0] },
            Vertex { position: v1.to_array(), normal: n2.to_array(), uv: [0.5, 1.0] },
            Vertex { position: v1.to_array(), normal: n3.to_array(), uv: [0.0, 0.0] },
            Vertex { position: v3.to_array(), normal: n3.to_array(), uv: [1.0, 0.0] },
            Vertex { position: v2.to_array(), normal: n3.to_array(), uv: [0.5, 1.0] },
        ];

        (vertices, (0..12).collect())
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
    if ((dist - 1.0) / sqrt(3.0) * radius < 0.0) { return 0.0; }
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
        let r = params.radius;
        let depth = water_height - center_y;

        if depth <= -r {
            0.0
        } else if depth >= r {
            self.volume(params)
        } else {
            let fraction = ((depth + r) / (2.0 * r)).clamp(0.0, 1.0);
            self.volume(params) * fraction
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::TetrahedronCollider::new())
    }
}
