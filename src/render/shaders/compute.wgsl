[[group(0), binding(0)]]
var r_texels: [[access(write)]] texture_storage_2d<rgba32float>;

const color: vec4<f32> = vec4<f32>(0.1, 0.2, 0.3, 1.0);

[[stage(compute), workgroup_size(32, 32)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) -> void {
    textureStore(r_texels, vec2<i32>(global_id.xy), color);
}