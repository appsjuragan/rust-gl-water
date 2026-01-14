//! Water surface rendering shader

use crate::shaders::common::COMMON_WGSL;

pub fn water_shader() -> String {
    format!(
        "{}\n{}",
        COMMON_WGSL,
        include_str!("wgsl/water_render.wgsl")
    )
}
