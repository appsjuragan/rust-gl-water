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
    shape_type: i32,
    _padding: f32,
    light_color: vec4<f32>,
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

// SDF Functions
fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdTorus(p: vec3<f32>, param: vec2<f32>) -> f32 {
    let t = vec2<f32>(param.x, param.y);
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

fn sdTetrahedron(p: vec3<f32>, r: f32) -> f32 {
    // scale down to unit
    let q = p / r;
    var md = max(abs(q.x + q.y) - q.z, abs(q.x - q.y) + q.z);
    md = max(md, abs(q.x) + abs(q.y) + abs(q.z) - 1.0 /* roughly */); 
    // Simplified octahedron/tetrahedron approximation
    return (length(q) - 0.7) * r; // Very rough proxy if exact formula is complex
    // Better Tetrahedron (IQ)
    // return (max(abs(q.x+q.y)-q.z, abs(q.x-q.y)+q.z) - 1.0)/sqrt(3.0) * r;
}

fn get_shape_dist(p: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    if (shape_type == 0) { // Sphere
        return sdSphere(p, radius);
    } else if (shape_type == 1) { // Torus
        // Outer radius approx 0.8 * radius, tube 0.3 * radius
        // The mesh was scaled by 'radius' in shader, so p is in world space unscaled?
        // No, p is usually relative to center.
        // The vertex shader transformed unit sphere by radius.
        // Mesh generation: Torus(0.7, 0.3).
        // So dimensions are roughly 1.0 unit.
        // We scale by radius.
        return sdTorus(p, vec2<f32>(0.7 * radius, 0.3 * radius));
    } else if (shape_type == 2) { // Tetrahedron
        // Mesh size 1.6 relative to unit sphere 1.0.
        return (max(abs(p.x+p.y)-p.z, abs(p.x-p.y)+p.z) - 1.0 * radius) / sqrt(3.0); 
    } else { // Cube
        return sdBox(p, vec3<f32>(0.7 * radius));
    }
}

// Raymarch to find exit interval or shadow
// Returns distance through object (0.0 if Miss)
fn intersect_shape_sdf(origin: vec3<f32>, ray: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    // Analytic sphere check first to optimize
    let r_bound = radius * 1.5; 
    let t_sphere = intersect_sphere(origin, ray, vec3<f32>(0.0), r_bound);
    if (t_sphere < 0.0) { return 0.0; }

    // March
    var t = 0.0;
    // Special case: we might be inside. 
    // For shadows (origin outside), start at sphere hit.
    // For refraction (origin on surface), start at 0.
    
    // Check if we are inside
    let d0 = get_shape_dist(origin, shape_type, radius);
    var inside = false;
    if (d0 < 0.0) { inside = true; }
    
    // Conservative marching
    var dist = 0.0;
    if (d0 > 0.0) { 
        // Outside, march to entry
        // Not implemented fully for generic shadow yet, rely on sphere proxy for perf
        // or simple march
    }
    
    return 0.0; // Placeholder
}

// Intersect ray with sphere (Analytic, needed for optimization)
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
    return -1.0;
}

// Get distance to exit point of the shape, assuming we are ON the surface or inside
fn get_exit_dist_shape(origin: vec3<f32>, dir: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    if (shape_type == 0) {
        // Sphere analytic
        // origin is relative to center 
        let a = dot(dir, dir);
        let b = 2.0 * dot(origin, dir);
        let c = dot(origin, origin) - radius * radius;
        // We expect one positive root since we are on surface/inside
        let discriminant = b*b - 4.0*a*c;
        if (discriminant >= 0.0) {
            return (-b + sqrt(discriminant)) / (2.0 * a);
        }
        return 0.0;
    }

    // Raymarch for others
    var t = 0.05 * radius; // Step away from surface
    for (var i = 0; i < 32; i++) {
        let p = origin + dir * t;
        let d = get_shape_dist(p, shape_type, radius);
        // If d > 0, we are outside. Since we started inside/on surface (approx), d becomes + when we exit.
        if (d > 0.001) {
            return t; 
        }
        // d is negative inside. We want to reach boundary (0).
        // Distance to boundary is abs(d).
        t += abs(d);
        if (t > radius * 3.0) { return 0.0; } // escaped bounds
    }
    return t;
}

// Shadow check (0.0 = occluded, 1.0 = lit)
fn get_shadow(origin: vec3<f32>, light_dir: vec3<f32>, shape_type: i32, radius: f32) -> f32 {
    // Analytic sphere shadow for all shapes for now (soft shadow proxy)
    // Or raymarch... Raymarching shadows is expensive.
    // Let's use Sphere proxy for shadows to keep perf high, visual difference is minor for caustics usually.
    // User complaint was "Refraction, reflection still a sphere". Reflection on surface is handled by normal (which comes from mesh).
    // Refraction is handled by ray tracing through object.
    
    // But for "ghost" on the floor, that's get_wall_color shadow.
    // Let's try to improve it slightly.
    
    return 1.0; // Placeholder, handled in get_wall_color
}

// Get sphere color with caustics (Renamed contextually, but kept name for compat if not changing call sites)
// Actually we will update call sites.
fn get_object_color_refraction(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>) -> vec3<f32> {
    
    // ... logic ...
    return vec3<f32>(0.0);
}

// Re-implement old get_sphere_color but with shape awareness?
// No, get_sphere_color was for rendering the sphere itself? No, sphere shader renders sphere.
// get_sphere_color was used to render WHAT?
// Checking common.rs content... "fn get_sphere_color...".
// Used by ... ? sphere shader uses "get_surface_ray_color".
// "get_sphere_color" seems unused in `sphere.rs`.
// It might be used by `water_render.rs` ?? To render sphere seen through water?
// Yes, `water_render.rs` raycasts and hits sphere.

fn intersect_shape_any(origin: vec3<f32>, ray: vec3<f32>, center: vec3<f32>, radius: f32, shape_type: i32) -> f32 {
    // Check bounding sphere first
    let bound_r = radius * 1.5;
    var t = 0.0;
    
    // Check if we are inside the bounding sphere
    let dist_sq = dot(origin - center, origin - center);
    if (dist_sq > bound_r * bound_r) {
        // Outside: check intersection with bounding sphere
        let t_sphere = intersect_sphere(origin, ray, center, bound_r);
        if (t_sphere < 0.0) { return -1.0; }
        t = max(0.0, t_sphere - 0.1);
    }
    
    // If shape is sphere, return analytic intersection
    if (shape_type == 0) { 
        if (dist_sq < radius * radius) { return 0.0; } // Inside sphere
        return intersect_sphere(origin, ray, center, radius); 
    }
    
    // Otherwise raymarch from t (entry point or 0.0) 
    // transform origin/ray to local space of shape
    let local_origin = origin - center;
    
    for (var i=0; i<32; i++) {
        let p = local_origin + ray * t;
        let d = get_shape_dist(p, shape_type, radius);
        if (d < 0.001) { return t; }
        t += d;
        if (t > radius * 3.0) { return -1.0; }
    }
    return -1.0;
}

// Get wall/floor color with caustics
fn get_wall_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>, tile_color: vec3<f32>) -> vec3<f32> {
    var scale = vec3<f32>(0.5);
    
    let pool_size = uniforms.pool_size;
    let sphere_center = uniforms.sphere_center.xyz;
    let sphere_radius = uniforms.sphere_radius;
    let shape_type = uniforms.shape_type;
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
    
    // scale /= length(point); // Removed to fix ghost shadow/vignette artifact
    
    let refracted_light = -refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    let diffuse = max(0.0, dot(refracted_light, normal));
    let lit_diffuse = diffuse * uniforms.light_color.rgb;
    
    // Calculate shadow using proper shape-aware intersection
    var shadow = 1.0;
    let hit_shape = intersect_shape_any(point, refracted_light, sphere_center, sphere_radius, shape_type);
    if (hit_shape > 0.0) { 
        shadow = 0.2; // In shadow
    }
    
    if point.y < water_info.r {
        // Underwater: apply caustics with shadow
        scale += lit_diffuse * caustic_sample.r * 2.0 * caustic_sample.g * shadow;
    } else {
        // Above water: use direct lighting with shadow
        scale += lit_diffuse * shadow * 0.5;
    }
    
    return wall_color * scale;
}

// Get sphere color (seen through water or directly)
// Used by water_render.rs to render the object when ray hits it
fn get_sphere_color(point: vec3<f32>, uniforms: CommonUniforms, water_info: vec4<f32>, caustic_sample: vec4<f32>) -> vec3<f32> {
    // This function is for coloring the object surface.
    // Simple Lit Color
    let sphere_center = uniforms.sphere_center.xyz;
    let sphere_radius = uniforms.sphere_radius;
    let light = uniforms.light_dir.xyz;
    
    // Normal? We don't have normal passed in 'point' only p.
    // For sphere normal is (p-c)/r. For others we need gradient of SDF.
    let local_p = point - sphere_center;
    // SDF Gradient for normal
    let e = 0.001;
    let shape_type = uniforms.shape_type;
    let dx = get_shape_dist(local_p + vec3<f32>(e,0.0,0.0), shape_type, sphere_radius) - get_shape_dist(local_p - vec3<f32>(e,0.0,0.0), shape_type, sphere_radius);
    let dy = get_shape_dist(local_p + vec3<f32>(0.0,e,0.0), shape_type, sphere_radius) - get_shape_dist(local_p - vec3<f32>(0.0,e,0.0), shape_type, sphere_radius);
    let dz = get_shape_dist(local_p + vec3<f32>(0.0,0.0,e), shape_type, sphere_radius) - get_shape_dist(local_p - vec3<f32>(0.0,0.0,e), shape_type, sphere_radius);
    let normal = normalize(vec3<f32>(dx, dy, dz));
    
    let refracted_light = refract(-light, vec3<f32>(0.0, 1.0, 0.0), IOR_AIR / IOR_WATER);
    var diffuse = max(0.0, dot(-refracted_light, normal)) * 0.5;
    
    if point.y < water_info.r {
        diffuse *= caustic_sample.r * 4.0;
    }
    
    return vec3<f32>(0.5) * diffuse * uniforms.light_color.rgb + vec3<f32>(0.2) * uniforms.light_color.rgb; // Ambient
}
"#;
