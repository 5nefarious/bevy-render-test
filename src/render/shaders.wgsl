[[builtin(vertex_index)]]
var<in> in_vertex_index: u32;
[[builtin(position)]]
var<out> out_pos: vec4<f32>;
[[location(0)]]
var<out> out_tex_coords: vec2<f32>;

[[stage(vertex)]]
fn vs_main() {
    var triangle: array<vec2<f32>, 3u> = array<vec2<f32>, 3u>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    out_pos = vec4<f32>(triangle[in_vertex_index], 0.0, 1.0);
    out_tex_coords = (triangle[in_vertex_index] + 1.0) / 2.0;
}

[[location(0)]]
var<in> in_tex_coords: vec2<f32>;
[[location(0)]]
var<out> out_color: vec4<f32>;

[[group(0), binding(0)]]
var r_color: texture_2d<f32>;
[[group(0), binding(1)]]
var r_sampler: sampler;

[[stage(fragment)]]
fn fs_main() {
    out_color = textureSample(r_color, r_sampler, in_tex_coords);
}