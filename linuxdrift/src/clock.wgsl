struct Rect { value: vec4<f32> };
@group(0) @binding(0) var clock_texture: texture_2d<f32>;
@group(0) @binding(1) var clock_sampler: sampler;
@group(0) @binding(2) var<uniform> rect: Rect;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) index: u32) -> Output {
    let corners = array<vec2<f32>, 6>(vec2(0.,0.), vec2(0.,1.), vec2(1.,0.), vec2(1.,0.), vec2(0.,1.), vec2(1.,1.));
    let uv = corners[index];
    var out: Output;
    out.position = vec4(rect.value.xy + uv * rect.value.zw, 0., 1.);
    out.uv = uv;
    return out;
}
@fragment fn fs(in: Output) -> @location(0) vec4<f32> {
    return textureSample(clock_texture, clock_sampler, in.uv);
}
