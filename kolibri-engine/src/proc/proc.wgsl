struct Static {
  scene_size: vec2f,
}

@group(0) @binding(0)
var<uniform> statics: Static;

@vertex
fn vx_main(@builtin(vertex_index) vix: u32) -> @builtin(position) vec4f {
  const verts = array(
    vec2( 1., -1.),
    vec2(-1., -1.),
    vec2( 1.,  1.),
    vec2(-1.,  1.),
  );
  return vec4(verts[vix], 0, 1);
}

// NOTE: Only contains vertex shader. Fragment Shader is provided by
// Scene caller
