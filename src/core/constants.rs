//! Global constants and configuration defaults\n
// Window & Display
pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 800;
pub const WINDOW_TITLE: &str = "Rust GL Water - demo of GL water";

// Physics Defaults
pub const DEFAULT_GRAVITY: f32 = 9.81;
pub const DEFAULT_OBJECT_RADIUS: f32 = 0.25;
pub const DEFAULT_OBJECT_COUNT: usize = 5;
pub const PHYSICS_TIMESTEP: f32 = 1.0 / 60.0;
pub const MAX_PHYSICS_SUBSTEPS: u32 = 6;

// Pool Dimensions (Default)
pub const POOL_SIZE_DEFAULT: f32 = 2.0;         // Width/Length for Cube/Tube
pub const POOL_SIZE_CUBOID_WIDTH: f32 = 2.0;
pub const POOL_SIZE_CUBOID_LENGTH: f32 = 3.0;
pub const POOL_DEPTH: f32 = 1.0;
pub const WALL_HEIGHT: f32 = 0.4;

// Frustum Pool Specifics
pub const FRUSTUM_TOP_SCALE: f32 = 1.0;
pub const FRUSTUM_BOTTOM_SCALE: f32 = 0.7;

// Damping & Forces
pub const DRAG_LINEAR: f32 = 0.5;
pub const DRAG_QUADRATIC: f32 = 2.0;
pub const ANGULAR_DAMPING: f32 = 0.98;
pub const VELOCITY_DAMPING: f32 = 0.995;
pub const CONTACT_DAMPING: f32 = 0.92;
pub const RESTITUTION: f32 = 0.1;
pub const FRICTION: f32 = 0.4;

// Rendering
pub const WATER_TEXTURE_SIZE: u32 = 512;
pub const CAUSTICS_TEXTURE_SIZE: u32 = 512;
pub const DEFAULT_LIGHT_DIR: [f32; 3] = [-1.0, 1.0, 1.0];
pub const DEFAULT_LIGHT_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
