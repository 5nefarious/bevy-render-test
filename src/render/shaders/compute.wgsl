var<private> color_fg: vec4<f32> = vec4<f32>(0.9, 0.8, 0.5, 1.0);
var<private> color_bg: vec4<f32> = vec4<f32>(0.01, 0.02, 0.05, 1.0);
var<private> camera_pos: vec3<f32> = vec3<f32>(0.0, 0.0, 5.0);
var<private> camera_size: f32 = 1.0;
var<private> camera_fov: f32 = 60.0;
var<private> max_iter: i32 = 20;
var<private> tol: f32 = 0.00001;

@group(0) @binding(0)

fn sphere_sdf(p: vec3<f32>, c: vec3<f32>, r: f32) -> f32 {
    return distance(p, c) - r;
}
var<private> sphere_center: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
var<private> sphere_radius: f32 = 1.0;

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let pixel_id = vec2<i32>(global_id.xy);
    let extent = textureDimensions(r_texels);
    if (all(pixel_id < extent)) {
        var color = color_bg;
        var pos = camera_pos;
        let fext = vec2<f32>(extent);
        let phi = atan2(fext.y, fext.x);
        let diag_dst = camera_size * tan(radians(camera_fov));
        let top_left = vec3<f32>(
            diag_dst / 2.0 * -cos(phi),
            diag_dst / 2.0 *  sin(phi),
            camera_pos.z - camera_size,
        );
        let diag_pxl = distance(fext, vec2<f32>());
        let pxl2dst = diag_dst / diag_pxl;
        let pixel_pos = top_left + (pxl2dst
            * vec3<f32>(global_id) * vec3<f32>(1.0, -1.0, 1.0));
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
        textureStore(r_texels, pixel_id, color);
    }
}