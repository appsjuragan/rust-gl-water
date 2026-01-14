//! Sphere shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams, ShapeVertex};
use glam::Vec3;
use std::f32::consts::PI;

/// Sphere shape
pub struct Sphere;

impl Sphere {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sphere {
    fn default() -> Self {
        Self::new()
    }
}

impl Shape for Sphere {
    fn name(&self) -> &str {
        "Sphere"
    }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        point.length() - params.radius
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>) {
        let radius = params.radius;
        let lat_segments = params.subdivisions;
        let lon_segments = params.subdivisions;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for lat in 0..=lat_segments {
            let theta = lat as f32 * PI / lat_segments as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for lon in 0..=lon_segments {
                let phi = lon as f32 * 2.0 * PI / lon_segments as f32;
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let x = sin_theta * cos_phi;
                let y = cos_theta;
                let z = sin_theta * sin_phi;

                let position = [x * radius, y * radius, z * radius];
                let normal = [x, y, z]; // Normalized by construction
                let uv = [lon as f32 / lon_segments as f32, lat as f32 / lat_segments as f32];

                vertices.push(ShapeVertex { position, normal, uv });
            }
        }

        // Generate indices
        for lat in 0..lat_segments {
            for lon in 0..lon_segments {
                let current = lat * (lon_segments + 1) + lon;
                let next = current + lon_segments + 1;

                indices.push(current);
                indices.push(next);
                indices.push(current + 1);

                indices.push(current + 1);
                indices.push(next);
                indices.push(next + 1);
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
            // Completely above water
            0.0
        } else if depth >= r {
            // Completely submerged
            self.volume(params)
        } else {
            // Partially submerged - use spherical cap formula
            let h = r + depth; // Height of submerged cap
            PI * h * h * (3.0 * r - h) / 3.0
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}
