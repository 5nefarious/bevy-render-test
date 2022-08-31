var<private> color: vec4<f32> = vec4<f32>(0.1, 0.2, 0.3, 1.0);

@group(0) @binding(0)
var r_texels: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    textureStore(r_texels, vec2<i32>(global_id.xy), color);
}