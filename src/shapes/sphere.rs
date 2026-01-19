//! Sphere shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams};
use crate::core::geometry::Vertex;
use glam::Vec3;
use std::f32::consts::PI;

pub struct Sphere;

impl Sphere {
    pub fn new() -> Self { Self }
}

impl Default for Sphere {
    fn default() -> Self { Self::new() }
}

impl Shape for Sphere {
    fn name(&self) -> &str { "Sphere" }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        point.length() - params.radius
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<Vertex>, Vec<u32>) {
        let radius = params.radius;
        let segments = params.subdivisions;

        let mut vertices = Vec::with_capacity(((segments + 1) * (segments + 1)) as usize);
        let mut indices = Vec::with_capacity((segments * segments * 6) as usize);

        for lat in 0..=segments {
            let theta = lat as f32 * PI / segments as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();

            for lon in 0..=segments {
                let phi = lon as f32 * 2.0 * PI / segments as f32;
                let (sin_phi, cos_phi) = phi.sin_cos();

                let x = sin_theta * cos_phi;
                let y = cos_theta;
                let z = sin_theta * sin_phi;

                vertices.push(Vertex {
                    position: [x * radius, y * radius, z * radius],
                    normal: [x, y, z],
                    uv: [lon as f32 / segments as f32, lat as f32 / segments as f32],
                });
            }
        }

        for lat in 0..segments {
            for lon in 0..segments {
                let current = lat * (segments + 1) + lon;
                let next = current + segments + 1;
                indices.extend_from_slice(&[current, next, current + 1, current + 1, next, next + 1]);
            }
        }

        (vertices, indices)
    }

    fn bounding_sphere(&self, params: &ShapeParams) -> (Vec3, f32) {
        (Vec3::ZERO, params.radius)
    }

    fn shader_intersection_code(&self) -> String {
        r#"
fn intersect_sphere(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, rotation: vec4<f32>) -> f32 {
    let to_sphere = origin - center;
    let b = dot(to_sphere, ray);
    let c = dot(to_sphere, to_sphere) - radius * radius;
    let d = b * b - c;
    if (d > 0.0) {
        let t = -b - sqrt(d);
        if (t > 0.0) { return t; }
    }
    return -1.0;
}
"#.to_string()
    }

    fn shader_sdf_code(&self) -> String {
        r#"
fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}
"#.to_string()
    }

    fn volume(&self, params: &ShapeParams) -> f32 {
        let r = params.radius;
        (4.0 / 3.0) * PI * r * r * r
    }

    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32 {
        let r = params.radius;
        let depth = water_height - center_y;

        if depth <= -r {
            0.0
        } else if depth >= r {
            self.volume(params)
        } else {
            let h = r + depth;
            PI * h * h * (3.0 * r - h) / 3.0
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}
