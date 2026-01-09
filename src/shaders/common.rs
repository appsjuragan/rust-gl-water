//! Common shader constants and helper functions

pub const COMMON_UNIFORMS: &str = r#"
// Index of refraction constants
const IOR_AIR: f32 = 1.0;
const IOR_WATER: f32 = 1.333;

// Water colors
const ABOVE_WATER_COLOR: vec3<f32> = vec3<f32>(0.6, 0.9, 1.0);
const UNDERWATER_COLOR: vec3<f32> = vec3<f32>(0.4, 0.9, 1.0);

// Common uniforms structure
struct CommonUniforms {
    pool_height: f32,
    wall_height: f32,
    pool_size: vec2<f32>,
    light_dir: vec4<f32>,
    sphere_center: vec4<f32>,
    sphere_radius: f32,
    time: f32,
    _padding: vec2<f32>,
}
"#;

pub const HELPER_FUNCTIONS: &str = r#"
// Intersect ray with axis-aligned box
fn intersect_cube(origin: vec3<f32>, ray: vec3<f32>, cube_min: vec3<f32>, cube_max: vec3<f32>) -> vec2<f32> {
    let t_min = (cube_min - origin) / ray;
    let t_max = (cube_max - origin) / ray;
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);
    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);
    return vec2<f32>(t_near, t_far);
}

// Intersect ray with sphere
fn intersect_sphere(origin: vec3<f32>, ray: vec3<f32>, sphere_center: vec3<f32>, sphere_radius: f32) -> f32 {
    let to_sphere = origin - sphere_center;
    let a = dot(ray, ray);
    let b = 2.0 * dot(to_sphere, ray);
    let c = dot(to_sphere, to_sphere) - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant > 0.0 {
        let t = (-b - sqrt(discriminant)) / (2.0 * a);
        if t > 0.0 {
            return t;
        }
    }
    return 1.0e6;
}

// Get sphere color with caustics
fn get_sphere_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>) -> vec3<f32> {
    var color = vec3<f32>(0.5);
    
    let pool_size = uniforms.pool_size;
    let sphere_center = uniforms.sphere_center.xyz;
    let sphere_radius = uniforms.sphere_radius;
    let pool_height = uniforms.pool_height;
    let light = uniforms.light_dir.xyz;
    
    color *= 1.0 - 0.9 / pow((pool_size.x + sphere_radius - abs(point.x)) / sphere_radius, 3.0);
    color *= 1.0 - 0.9 / pow((pool_size.y + sphere_radius - abs(point.z)) / sphere_radius, 3.0);
    color *= 1.0 - 0.9 / pow((point.y + pool_height + sphere_radius) / sphere_radius, 3.0);
    
    let sphere_normal = (point - sphere_center) / sphere_radius;
    let refracted_light = refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    var diffuse = max(0.0, dot(-refracted_light, sphere_normal)) * 0.5;
    
    if point.y < water_info.r {
        diffuse *= caustic_sample.r * 4.0;
    }
    color += vec3<f32>(diffuse);
    
    return color;
}

// Get wall/floor color with caustics
fn get_wall_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>, tile_color: vec3<f32>) -> vec3<f32> {
    var scale = 0.5;
    
    let pool_size = uniforms.pool_size;
    let sphere_center = uniforms.sphere_center.xyz;
    let sphere_radius = uniforms.sphere_radius;
    let pool_height = uniforms.pool_height;
    let wall_height = uniforms.wall_height;
    let light = uniforms.light_dir.xyz;
    
    var wall_color = tile_color;
    var normal: vec3<f32>;
    
    if abs(point.x) > pool_size.x - 0.01 {
        normal = vec3<f32>(-sign(point.x), 0.0, 0.0);
    } else if abs(point.z) > pool_size.y - 0.01 {
        normal = vec3<f32>(0.0, 0.0, -sign(point.z));
    } else {
        normal = vec3<f32>(0.0, 1.0, 0.0);
    }
    
    scale /= length(point);
    scale *= 1.0 - 0.9 / pow(length(point - sphere_center) / sphere_radius, 4.0);
    
    let refracted_light = -refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    let diffuse = max(0.0, dot(refracted_light, normal));
    
    if point.y < water_info.r {
        scale += diffuse * caustic_sample.r * 2.0 * caustic_sample.g;
    } else {
        let cube_min = vec3<f32>(-pool_size.x, -pool_height, -pool_size.y);
        let cube_max = vec3<f32>(pool_size.x, wall_height, pool_size.y);
        let t = intersect_cube(point, refracted_light, cube_min, cube_max);
        let shadow = 1.0 / (1.0 + exp(-200.0 / (1.0 + 10.0 * (t.y - t.x)) * (point.y + refracted_light.y * t.y - wall_height)));
        scale += diffuse * shadow * 0.5;
    }
    
    return wall_color * scale;
}
"#;
