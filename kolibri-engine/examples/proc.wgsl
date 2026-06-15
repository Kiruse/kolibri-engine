@group(0) @binding(0)
var<uniform> statics: Static;

@group(0) @binding(1)
var<uniform> timings: Timings;

struct Static {
  scene_size: vec2f,
}

struct Timings {
  delta_time: f32,
  world_time: f32,
  scene_time: f32,
}

@fragment
fn fx_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let res = statics.scene_size;
  let uv = vec2((pos.x / res.x) * 2. - 1., 1. - (pos.y / res.y) * 2.);
  let scene_pos = vec2(uv.x * res.x, uv.y * res.y) / 2;
  let line_thickness = 5.;

  let c = 1. - abs(sdf_head(scene_pos) / line_thickness);
  return vec4(c, c, c, 1);
}

/// Compute distance of `pos` from cat head (excluding ears).
fn sdf_head(pos: vec2f) -> f32 {
  var d = sdf_circle(pos, vec2(0, 0), 500.);
  d = min(d, sdf_ear_l(pos));
  d = min(d, sdf_ear_r(pos));
  return d;
}

/// Compute distance of `pos` from left cat ear surface.
fn sdf_ear_l(pos: vec2f) -> f32 {
  return sdf_tri(pos, vec2(450, 600), vec2(450, 200), vec2(0, 300));
}

/// Compute distance of `pos` from the right cat ear surface.
fn sdf_ear_r(pos: vec2f) -> f32 {
  return sdf_tri(pos, vec2(-450, 600), vec2(-450, 200), vec2(0, 300));
}

/// Compute the distance of `pos` from the surface of the triangle
/// specified by the given points.
fn sdf_tri(pos: vec2f, p0: vec2f, p1: vec2f, p2: vec2f) -> f32 {
  var d = sdf_line(pos, p0, p1, p2);
  d = max(d, sdf_line(pos, p1, p2, p0));
  d = max(d, sdf_line(pos, p2, p0, p1));
  return d;
}

/// Compute the distance of `pos` from the given circle's circumference.
fn sdf_circle(pos: vec2f, c: vec2f, r: f32) -> f32 {
  let pos1 = pos - c;
  let d = length(pos1);
  return d - r;
}

/// Compute the distance of `pos` from the line formed by given vectors, using `r` to indicate
/// which side should be considered "inside" i.e. negative.
fn sdf_line(pos: vec2f, p0: vec2f, p1: vec2f, r: vec2f) -> f32 {
  let a = p1.y - p0.y;
  let b = p0.x - p1.x;
  let c = a * p0.x + b * p0.y;
  let p = a * pos.x + b * pos.y - c;   // direct form, no double subtraction
  let n = length(vec2(a, b));
  let s = -sign(a * r.x + b * r.y - c) * sign(p);
  return s * abs(p) / n;
}
