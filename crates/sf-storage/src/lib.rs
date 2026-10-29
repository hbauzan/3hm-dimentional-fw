//! LanceDB persistence for domain hypercubes.

mod calibration;
mod lance_store;
mod mem_registry;
mod registry;
mod schema;
mod validate;

pub use calibration::{
    calibrate_matrix, calibrate_noise_bitmask, read_f32_matrix, Calibration, CalibrationError,
};
pub use lance_store::{load_registry, write_calibration, IngestSpec};
pub use mem_registry::MemoryRegistry;
pub use registry::{record_batch_from_calibration, DomainRegistry, HotRegistry, RegistryError};
pub use schema::{domain_hypercube_arrow_schema, TABLE_NAME};
pub use sf_kernels::DIM;
pub use validate::{load_validated_vec1024, LoadError, UNALIGNED_ARROW_BUFFER_LOADS};
