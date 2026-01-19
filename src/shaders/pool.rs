//! Pool rendering shader - renders the floor and walls of the pool

use crate::shaders::common::COMMON_WGSL;

pub fn pool_shader() -> String {
    format!(
        "{}\n{}",
        COMMON_WGSL,
        include_str!("wgsl/pool.wgsl")
    )
}
