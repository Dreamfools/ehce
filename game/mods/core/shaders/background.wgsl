#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> offset: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let offset = offset.xy;
    let trimmed = mesh.world_position.xy + offset;
    let trimmed_mod = trimmed - floor(trimmed);

    return select(vec4<f32>(0.0, 0.0, 0.0, 0.0), vec4<f32>(0.1, 0.1, 0.1, 0.0), max(trimmed_mod.x, trimmed_mod.y) < 0.1);
}
