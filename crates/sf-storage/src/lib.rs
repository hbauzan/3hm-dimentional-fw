//! Storage facade. Slice 1 exposes the in-memory registry; LanceDB arrives in Slice 3.

mod mem_registry;

pub use mem_registry::MemoryRegistry;
pub use sf_kernels::DIM;
