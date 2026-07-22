Kolibri Engine is an experimental procedural 3D rendering engine built for personal learning purposes. It is written in Rust and targets modern GPUs via [wgpu](https://wgpu.rs/), with [winit](https://github.com/rust-windowing/winit) for windowing, [egui](https://github.com/emilk/egui) for immediate-mode UI, and [glam](https://github.com/bitshifter/glam-rs) for linear algebra.

## Overview

The engine centers on a **procedural rendering** model rather than a traditional entity-component one. Fragment shaders are authored in WGSL. HMR is an experimental feature for `ProceduralScene`s but may be reused in other scenes as well where logical.

Rendering is structured around the `Scene` trait — each scene owns its pipeline, manages its own resources, and draws into a wgpu render pass.

- `ProceduralScene` is the simplest scene which provides a fixed vertex shader and takes a fragment shader factory. The fragment shader receives a fixed uniform layout from the `ProceduralScene`.
- `VoxelScene` is the current WIP, the main focus of work on this project, and its first high level abstraction with built-in optimizations. The virtual world uses a `std::collections::HashMap<[i32; 3], OctreeRoot>` to divide the world into *up to* 32x32x32 chunks of voxels.

## Running Examples

Requires a GPU and a Rust toolchain (edition 2024).

```sh
cargo run -p kolibri-engine --example <example_name>
```

There are currently 3 examples:

- `proc` serves as minimal reference implementation for 2D SDF-based scenes.
- `cube` serves as minimal reference implementation for 3D SDF-based scenes.
- `voxel` (WIP) is the first higher level abstraction that implements voxel
  terrain through sparse octree & arbitrary loose geometry.

Developed in this order over the lifetime of the project.

## Roadmap

- [x] Basic `proc` example
- [x] Basic `cube` example
- [ ] Basic single octree-based voxel chunk rendering <- in progress
- [ ] Multi-chunk rendering + frustum culling
- [ ] Chunk occlusion culling
- [ ] Octree 3D texture baking
- [ ] `ProceduralScene` abstractions

## License

Currently proprietary — not intended for public use. License will change as the project matures and stabilizes.
