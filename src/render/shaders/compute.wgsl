var<private> color_fg: vec4<f32> = vec4<f32>(0.9, 0.8, 0.5, 1.0);
var<private> color_bg: vec4<f32> = vec4<f32>(0.01, 0.02, 0.05, 1.0);
var<private> camera_pos: vec3<f32> = vec3<f32>(0.0, 0.0, 5.0);
var<private> camera_size: f32 = 1.0;
var<private> camera_fov: f32 = 60.0;
var<private> max_iter: i32 = 20;
var<private> tol: f32 = 0.00001;


struct RaySamplingUniform {
    seed: vec2<u32>,
    extent: vec2<u32>,
};

@group(0) @binding(0)
var r_texels: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<uniform> ray_sampling: RaySamplingUniform;


fn sphere_sdf(p: vec3<f32>, c: vec3<f32>, r: f32) -> f32 {
    return distance(p, c) - r;
}
var<private> sphere_center: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
var<private> sphere_radius: f32 = 1.0;


fn pcg2d(key: vec2<u32>) -> vec2<u32> {
    let a = 1664525u;
    let b = 1013904223u;
    var v = key * a + b;
    v.x += v.y * a;
    v.y += v.x * a;
    v ^= (v >> vec2<u32>(16u, 16u));
    v.x += v.y * a;
    v.y += v.x * a;
    v ^= (v >> vec2<u32>(16u, 16u));
    return v;
}

fn construct_float(m: u32) -> f32 {
    let mantissa: u32   = 0x007fffffu;
    let one: u32        = 0x3f800000u;
    var b: u32 = m;
    b &= mantissa;
    b |= one;
    return bitcast<f32>(b) - 1.0;
}

fn uniform2d(key: vec2<u32>) -> vec2<f32> {
    let hash = pcg2d(key);
    return vec2<f32>(
        construct_float(hash.x),
        construct_float(hash.y),
    );
}

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let extent = min(
        vec2<f32>(ray_sampling.extent),
        vec2<f32>(textureDimensions(r_texels)),
    );
    let ray_seed = ray_sampling.seed + global_id.xy;
    let patch_ext = extent / vec2<f32>(num_workgroups.xy);
    let patch_pos = patch_ext * uniform2d(ray_seed);
    let global_pos = patch_pos + patch_ext * vec2<f32>(workgroup_id.xy);

    var color = color_bg;
    var pos = camera_pos;
    let phi = atan2(extent.y, extent.x);
    let diag_dst = camera_size * tan(radians(camera_fov));
    let top_left = vec3<f32>(
        diag_dst / 2.0 * -cos(phi),
        diag_dst / 2.0 *  sin(phi),
        camera_pos.z - camera_size,
    );
    let diag_pxl = distance(extent, vec2<f32>());
    let pxl2dst = diag_dst / diag_pxl;
    let pixel_pos = top_left + pxl2dst * vec3<f32>(
         global_pos.x,
        -global_pos.y,
         0.0,
    );
    let dir = normalize(pixel_pos - camera_pos);
    for (var i = 0; i < max_iter; i++) {
        let dist = sphere_sdf(pos, sphere_center, sphere_radius);
        if (dist < tol) {
            color = color_fg;
            break;
        } else {
            pos += dir * dist;
        }
    }
    textureStore(r_texels, vec2<i32>(global_pos), color);
}