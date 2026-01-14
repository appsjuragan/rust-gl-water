//! Sphere rendering shader - ray-marched object rendering with material properties

use crate::shaders::common::COMMON_WGSL;

pub fn sphere_shader() -> String {
    format!(
        "{}\n{}",
        COMMON_WGSL,
        include_str!("wgsl/sphere.wgsl")
    )
}
