use encase::{ShaderType, UniformBuffer, internal::WriteInto};

/// Generalized helper function to serialize any `encase::ShaderType` into
/// a `Vec<u8>`. As such, it may not be optimal for all use cases.
pub fn to_buffer<T: ShaderType + WriteInto>(data: &T) -> Vec<u8> {
  let mut buffer = UniformBuffer::new(Vec::<u8>::new());
  buffer.write(data).unwrap();
  buffer.into_inner()
}
