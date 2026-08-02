#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> transform: vec3<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let offset = transform.xy;
    let scale = transform.z;
    var trimmed = (mesh.uv + offset) * scale % 1;
    let trimmed2 = (mesh.uv + offset) * scale % 2;

    return select(vec4<f32>(0.0, 0.0, 0.0, 0.0), vec4<f32>(0.1, 0.1, 0.1, 0.0), max(trimmed.x, trimmed.y) < 0.1);
}
