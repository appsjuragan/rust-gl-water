//! Torus shape implementation

use crate::core::shape::{Shape, ShapeParams, MeshParams, ShapeVertex};
use glam::Vec3;
use std::f32::consts::PI;

pub struct Torus;

impl Torus {
    pub fn new() -> Self { Self }
    
    const MAJOR_RATIO: f32 = 0.7;
    const MINOR_RATIO: f32 = 0.3;
}

impl Default for Torus {
    fn default() -> Self { Self::new() }
}

impl Shape for Torus {
    fn name(&self) -> &str { "Torus" }

    fn sdf(&self, point: Vec3, params: &ShapeParams) -> f32 {
        let major_r = params.radius * Self::MAJOR_RATIO;
        let minor_r = params.radius * Self::MINOR_RATIO;
        let q_x = (point.x * point.x + point.z * point.z).sqrt() - major_r;
        Vec3::new(q_x, point.y, 0.0).length() - minor_r
    }

    fn generate_mesh(&self, params: &MeshParams) -> (Vec<ShapeVertex>, Vec<u32>) {
        let major_r = params.radius * Self::MAJOR_RATIO;
        let minor_r = params.radius * Self::MINOR_RATIO;
        let radial_seg = params.subdivisions;
        let tubular_seg = params.subdivisions_secondary.unwrap_or(params.subdivisions);

        let vert_count = ((radial_seg + 1) * (tubular_seg + 1)) as usize;
        let idx_count = (radial_seg * tubular_seg * 6) as usize;
        let mut vertices = Vec::with_capacity(vert_count);
        let mut indices = Vec::with_capacity(idx_count);

        for i in 0..=radial_seg {
            let u = i as f32 / radial_seg as f32 * 2.0 * PI;
            let (sin_u, cos_u) = u.sin_cos();

            for j in 0..=tubular_seg {
                let v = j as f32 / tubular_seg as f32 * 2.0 * PI;
                let (sin_v, cos_v) = v.sin_cos();

                vertices.push(ShapeVertex {
                    position: [
                        (major_r + minor_r * cos_v) * cos_u,
                        minor_r * sin_v,
                        (major_r + minor_r * cos_v) * sin_u,
                    ],
                    normal: [cos_v * cos_u, sin_v, cos_v * sin_u],
                    uv: [i as f32 / radial_seg as f32, j as f32 / tubular_seg as f32],
                });
            }
        }

        for i in 0..radial_seg {
            for j in 0..tubular_seg {
                let a = i * (tubular_seg + 1) + j;
                let b = a + tubular_seg + 1;
                indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
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
    if (sdTorus(local_origin, vec2<f32>(major_r, minor_r)) < 0.0) { return 0.0; }
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
        let major_r = params.radius * Self::MAJOR_RATIO;
        let minor_r = params.radius * Self::MINOR_RATIO;
        2.0 * PI * PI * major_r * minor_r * minor_r
    }

    fn submerged_volume(&self, params: &ShapeParams, center_y: f32, water_height: f32) -> f32 {
        let minor_r = params.radius * Self::MINOR_RATIO;
        let depth = water_height - center_y;

        if depth <= -minor_r {
            0.0
        } else if depth >= minor_r {
            self.volume(params)
        } else {
            let fraction = ((depth + minor_r) / (2.0 * minor_r)).clamp(0.0, 1.0);
            self.volume(params) * fraction
        }
    }

    fn collider(&self) -> Box<dyn crate::core::physics_trait::Collider> {
        Box::new(crate::scene::collider_impl::BoundingSphereCollider::new())
    }
}
