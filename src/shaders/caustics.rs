//! Caustics shader - renders light caustics on the pool floor

use crate::shaders::common::COMMON_WGSL;

pub fn caustics_shader() -> String {
    format!(
        "{}\n{}",
        COMMON_WGSL,
        include_str!("wgsl/caustics.wgsl")
    )
}
