//! Numerical kernel surface. Slice 0 keeps the crate linkable; gates land in later slices.
#![forbid(unsafe_code)]

/// Contract dimension for `BAAI/bge-m3` dense vectors.
pub const DIM: usize = 1024;
