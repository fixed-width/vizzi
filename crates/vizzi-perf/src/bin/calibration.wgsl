@group(0) @binding(0) var<uniform> camera: vec4f;

struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
}

@vertex fn node_vs(@location(0) rect: vec4f, @builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOut {
    let corners = array<vec2f, 6>(vec2f(-1,-1), vec2f(1,-1), vec2f(-1,1), vec2f(-1,1), vec2f(1,-1), vec2f(1,1));
    var out: VertexOut;
    out.position = vec4f((rect.xy + corners[vertex] * rect.zw) * camera.x + camera.yz, 0, 1);
    out.color = select(vec4f(0.1,0.7,0.5,1), vec4f(0.9,0.7,0.2,1), instance % 37 == 0);
    return out;
}

@vertex fn edge_vs(@location(0) position: vec2f) -> VertexOut {
    var out: VertexOut;
    out.position = vec4f(position * camera.x + camera.yz, 0, 1);
    out.color = vec4f(0.3,0.6,0.7,0.15);
    return out;
}

@fragment fn fragment(in: VertexOut) -> @location(0) vec4f {
    return in.color;
}
