//! Water simulation shaders - GPGPU compute/fragment shaders for wave simulation

use crate::shaders::common::COMMON_WGSL;

/// Drop addition shader - adds ripples at a point
pub const DROP_SHADER: &str = include_str!("wgsl/water_drop.wgsl");

/// Wave update shader - propagates waves using wave equation
pub const UPDATE_SHADER: &str = include_str!("wgsl/water_update.wgsl");

/// Normal calculation shader
pub const NORMAL_SHADER: &str = include_str!("wgsl/water_normal.wgsl");

/// Sphere volume displacement shader
pub fn sphere_volume_shader() -> String {
    format!(
        "{}\n{}",
        COMMON_WGSL,
        include_str!("wgsl/water_displacement.wgsl")
    )
}
