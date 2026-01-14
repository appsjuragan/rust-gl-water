//! Torus shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams, ShapeVertex};
use glam::Vec3;
use std::f32::consts::PI;

/// Torus shape
pub struct Torus;

impl Torus {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Torus {
    fn default() -> Self {
        Self::new()
    }
}

impl Shape for Torus {
    fn name(&self) -> &str {
        "Torus"
    }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        let major_radius = params.radius * 0.7;
        let minor_radius = params.radius * 0.3;
        let q_x = (point.x * point.x + point.z * point.z).sqrt() - major_radius;
        let q = Vec3::new(q_x, point.y, 0.0);
        q.length() - minor_radius
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>) {
        let major_radius = params.radius * 0.7;
        let minor_radius = params.radius * 0.3;
        let radial_segments = params.subdivisions;
        let tubular_segments = params.subdivisions_secondary.unwrap_or(params.subdivisions);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..=radial_segments {
            let u = i as f32 / radial_segments as f32 * 2.0 * PI;
            let cos_u = u.cos();
            let sin_u = u.sin();

            for j in 0..=tubular_segments {
                let v = j as f32 / tubular_segments as f32 * 2.0 * PI;
                let cos_v = v.cos();
                let sin_v = v.sin();

                let x = (major_radius + minor_radius * cos_v) * cos_u;
                let y = minor_radius * sin_v;
                let z = (major_radius + minor_radius * cos_v) * sin_u;

                let nx = cos_v * cos_u;
                let ny = sin_v;
                let nz = cos_v * sin_u;

                vertices.push(ShapeVertex {
                    position: [x, y, z],
                    normal: [nx, ny, nz],
                    uv: [i as f32 / radial_segments as f32, j as f32 / tubular_segments as f32],
                });
            }
        }

        for i in 0..radial_segments {
            for j in 0..tubular_segments {
                let a = i * (tubular_segments + 1) + j;
                let b = a + tubular_segments + 1;

                indices.push(a);
                indices.push(b);
                indices.push(a + 1);

                indices.push(b);
                indices.push(b + 1);
                indices.push(a + 1);
            }
        }

        (vertices, indices)
    }

    fn bounding_sphere(&self, params: &ShapeParams) -> (Vec3, f32) {
        (Vec3::ZERO, params.radius)
    }

    fn shader_intersection_code(&self) -> String {
        r#"
fn intersect_torus(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, rotation: vec4<f32>) -> f32 {
    let major_r = radius * 0.7;
    let minor_r = radius * 0.3;
    
    let inv_rotation = vec4<f32>(-rotation.xyz, rotation.w);
    let local_origin = rotate_vector(origin - center, inv_rotation);
    let local_ray = rotate_vector(ray, inv_rotation);
    
    if (sdTorus(local_origin, vec2<f32>(major_r, minor_r)) < 0.0) {
        return 0.0;
    }
    
    var t = 0.0;
    for (var i=0; i<64; i++) {
        let p = local_origin + local_ray * t;
        let d = sdTorus(p, vec2<f32>(major_r, minor_r));
        if (d < 0.0005) { return t; }
        t += d;
        if (t > radius * 4.0) { return -1.0; }
    }
    return -1.0;
}
"#.to_string()
    }

    fn shader_sdf_code(&self) -> String {
        r#"
fn sdTorus(p: vec3<f32>, t_param: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t_param.x, p.y);
    return length(q) - t_param.y;
}
"#.to_string()
    }

    fn volume(&self, params: &ShapeParams) -> f32 {
        let major_radius = params.radius * 0.7;
        let minor_radius = params.radius * 0.3;
        2.0 * PI * PI * major_radius * minor_radius * minor_radius
    }

    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32 {
        let minor_radius = params.radius * 0.3;
        let depth = water_height - center_y;

        if depth <= -minor_radius {
            0.0
        } else if depth >= minor_radius {
            self.volume(params)
        } else {
            let fraction = (depth + minor_radius) / (2.0 * minor_radius);
            self.volume(params) * fraction.clamp(0.0, 1.0)
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}
