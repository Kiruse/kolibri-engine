@group(0) @binding(0)
var<uniform> statics: Static;

@group(0) @binding(1)
var<uniform> timings: Timings;

@group(0) @binding(2)
var<uniform> camera: Camera;

struct Static {
  scene_size: vec2f,
}

struct Timings {
  delta_time: f32,
  world_time: f32,
  scene_time: f32,
}

struct Camera {
  rotation: mat3x3f,
  location: vec3f,
  tan_half_fov: f32, // tan(fov/2)
  aspect: f32,
}

// Rotation speed in radians per second
const ROTATION_SPEED = 0.25;
const MAX_MARCH_STEPS = 100;
const MAX_RAY_DIST = 1000;
const DISTANCE_THRESHOLD = 0.001;
const PI = 3.141592653589793;

@fragment
fn fx_main(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let res = statics.scene_size;
  let uv = vec2((pos.x / res.x) * 2. - 1., 1. - (pos.y / res.y) * 2.);

  let d = raymarch(ray(uv));
  if d > 2*DISTANCE_THRESHOLD {
    return vec4(0, 0, 0, 1);
  }
  let c = 1 + d / 50;
  return vec4(vec3(c) * 0.7, 1);
}

fn raymarch(ray: vec3f) -> f32 {
  var t = 0.;
  for (var i = 0; i < MAX_MARCH_STEPS; i += 1) {
    let p = ray * t + camera.location;
    let d = sdf_cube(p, vec3(0., 0., 10.), 4);
    if d < DISTANCE_THRESHOLD {
      return d;
    }

    t += d;
    if t > MAX_RAY_DIST {
      break;
    }
  }
  return MAX_RAY_DIST;
}

/// Computes the distance between `pg` and the surface of the cube at `origin`
/// with equilateral side length `s`.
/// Early dropout when `length(pg - origin) > 1.25s` with simple distance from origin.
fn sdf_cube(pg: vec3f, origin: vec3f, s: f32) -> f32 {
  let theta = -PI * (ROTATION_SPEED * timings.scene_time % 2);
  let rot = mat3x3(
    vec3(cos(theta), 0., sin(theta)),
    vec3(0., 1., 0.),
    vec3(-sin(theta), 0., cos(theta)),
  );
  let pl = abs(rot * (pg - origin));
  let corner = vec3(s / 2);
  let d = pl - corner;
  return length(max(d, vec3(0.))) + min(0, max(d.x, max(d.y, d.z)));
}

fn ray(uv: vec2f) -> vec3f {
  let px = uv.x * camera.tan_half_fov * camera.aspect;
  let py = uv.y * camera.tan_half_fov;
  let v = vec3(px, py, 1.);
  return normalize(camera.rotation * v);
}
