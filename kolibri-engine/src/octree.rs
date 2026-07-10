use crate::error::EngineError;

#[derive(Clone)]
pub enum Octree {
  Leaf(u32),
  Node(OctreeNode),
}

impl Default for Octree {
  fn default() -> Self {
    Self::Leaf(0)
  }
}

impl Octree {
  /// Convert this Octree to binary wgpu buffer contents
  pub fn to_buffer(&self) -> Vec<u8> {
    let buffers = self.buffers();
    todo!()
  }

  fn buffers(&self) -> Vec<OctreeBuffer> {
    todo!()
  }
}

#[derive(Clone)]
pub struct OctreeNode(Box<[Octree; 8]>);

impl Default for OctreeNode {
  fn default() -> Self {
    Self(Box::new([(); 8].map(|_| Octree::Leaf(0))))
  }
}

impl OctreeNode {
  pub fn set_octant(&mut self, octant: Octant, value: Octree) {
    self.0[octant.idx()] = value;
  }

  pub fn clear_octant(&mut self, octant: Octant) {
    self.0[octant.idx()] = Octree::Leaf(0);
  }
}

struct OctreeBuffer {
  voxel: u32,
  size: f32,
  origin: [f32; 3], // origin in world coordinates of this node, bottom front left corner
}

/// Coordinates of a voxel within a chunk, from (0, 0, 0) to (31, 31, 31).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkCoord([u8; 3]);

impl ChunkCoord {
  pub fn new(x: u8, y: u8, z: u8) -> Result<Self, EngineError> {
    let vals = [x, y, z];
    for v in vals {
      if v >= 32 {
        return Err(EngineError::input(format!("out of bounds: 0..32, got: {v}")));
      }
    }
    Ok(Self([x, y, z]))
  }

  /// Converts this chunk coords into a series of [Octant]s for [crate::octree::Octree]s.
  pub fn to_octants(&self) -> [Octant; 5] {
    let mut octants: [Octant; 5] = Default::default();
    let mut coords = self.0;
    // Effectively, each coordinate is split across the octants array & encodes the same coordinate in binary
    for i in 0..5 {
      let bs: [_; 3] = std::array::from_fn(|j| coords[j] & 1u8 << (4-i) != 0);
      coords = coords.map(|v| v & !(1u8 << (4-i)));
      octants[i] = Octant(bs);
    }
    octants
  }
}

/// 3D octant descriptor using 3 boolean values (x,y,z) used to
/// navigate sparse 3D octrees.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Octant([bool; 3]);

impl Octant {
  pub fn new(x: bool, y: bool, z: bool) -> Self {
    Self([x, y, z])
  }

  pub fn idx(&self) -> usize {
    self.0[0] as usize + self.0[1] as usize * 2 + self.0[2] as usize * 4
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_to_octants() {
    let coord = ChunkCoord::new(31, 31, 31).unwrap();
    assert_eq!(
      coord.to_octants(),
      std::array::from_fn(|_| Octant([true, true, true])),
    );

    let coord = ChunkCoord::new(31, 12, 18).unwrap();
    assert_eq!(
      coord.to_octants(),
      [
        // bin: 31,   12,    18
        Octant([true, false, true]),
        Octant([true, true, false]),
        Octant([true, true, false]),
        Octant([true, false, true]),
        Octant([true, false, false]),
      ],
    );

    let coord = ChunkCoord::new(2, 4, 17).unwrap();
    assert_eq!(
      coord.to_octants(),
      [
        // bin: 2,     4,     17
        Octant([false, false, true]),
        Octant([false, false, false]),
        Octant([false, true, false]),
        Octant([true, false, false]),
        Octant([false, false, true]),
      ],
    );
  }
}
