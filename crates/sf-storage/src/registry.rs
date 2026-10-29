//! RecordBatch ↔ domain lock, plus the lock-free hot registry.

use crate::schema::domain_hypercube_arrow_schema;
use crate::validate::{load_validated_vec1024, LoadError};
use crate::Calibration;
use arc_swap::ArcSwap;
use lancedb::arrow::arrow_array::{
    Array, FixedSizeBinaryArray, FixedSizeListArray, Float32Array, RecordBatch, StringArray,
    UInt16Array, UInt64Array, UInt8Array,
};
use lancedb::arrow::arrow_buffer::Buffer;
use lancedb::arrow::arrow_schema::Field;
use sf_kernels::{
    AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024, DEFAULT_QUORUM_K, DIM,
    MAX_TOPK, NOISE_MASK_WORDS,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct RegistryError {
    pub message: String,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RegistryError {}

impl From<LoadError> for RegistryError {
    fn from(err: LoadError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DomainRegistry {
    pub locks_by_id: HashMap<String, Arc<DomainHypercubeLock>>,
    pub all_locks: Vec<Arc<DomainHypercubeLock>>,
}

impl DomainRegistry {
    pub fn from_record_batches(batches: Vec<RecordBatch>) -> Result<Self, RegistryError> {
        let mut locks_by_id = HashMap::new();
        let mut all_locks = Vec::new();
        for batch in batches {
            append_batch(&batch, &mut locks_by_id, &mut all_locks)?;
        }
        Ok(Self {
            locks_by_id,
            all_locks,
        })
    }

    pub fn get(&self, domain_id: &str) -> Option<Arc<DomainHypercubeLock>> {
        self.locks_by_id.get(domain_id).cloned()
    }
}

pub struct HotRegistry {
    inner: ArcSwap<DomainRegistry>,
}

impl HotRegistry {
    pub fn new(initial: Arc<DomainRegistry>) -> Self {
        Self {
            inner: ArcSwap::from(initial),
        }
    }

    pub fn load(&self) -> arc_swap::Guard<Arc<DomainRegistry>> {
        self.inner.load()
    }

    pub fn store(&self, next: Arc<DomainRegistry>) {
        self.inner.store(next);
    }
}

pub struct LockMeta<'a> {
    pub domain_id: &'a str,
    pub version: u64,
    pub polarity: u8,
    pub l1: f32,
    pub l2: f32,
    pub epsilon: f32,
    pub quorum_k: u16,
}

pub fn record_batch_from_calibration(
    meta: LockMeta<'_>,
    calibration: &Calibration,
) -> Result<RecordBatch, RegistryError> {
    let schema = domain_hypercube_arrow_schema();
    let centroid = list_f32(&calibration.centroid)?;
    let lo = list_f32(&calibration.lo)?;
    let hi = list_f32(&calibration.hi)?;
    let top_k = list_u16(&calibration.top_k)?;
    let mask = mask_array(&calibration.noise_mask)?;
    let digest = FixedSizeBinaryArray::try_new(32, Buffer::from(calibration.sha256.to_vec()), None)
        .map_err(|err| RegistryError {
            message: err.to_string(),
        })?;
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec![meta.domain_id.to_string()])),
            Arc::new(UInt64Array::from(vec![meta.version])),
            Arc::new(UInt8Array::from(vec![meta.polarity])),
            Arc::new(Float32Array::from(vec![meta.l1])),
            Arc::new(Float32Array::from(vec![meta.l2])),
            Arc::new(Float32Array::from(vec![meta.epsilon])),
            Arc::new(UInt16Array::from(vec![meta.quorum_k])),
            Arc::new(centroid),
            Arc::new(lo),
            Arc::new(hi),
            Arc::new(top_k),
            Arc::new(mask),
            Arc::new(UInt64Array::from(vec![calibration.rows])),
            Arc::new(digest),
        ],
    )
    .map_err(|err| RegistryError {
        message: err.to_string(),
    })
}

fn list_f32(values: &[f32; DIM]) -> Result<FixedSizeListArray, RegistryError> {
    let child = Float32Array::from(values.to_vec());
    FixedSizeListArray::try_new(
        Arc::new(Field::new(
            "item",
            lancedb::arrow::arrow_schema::DataType::Float32,
            false,
        )),
        DIM as i32,
        Arc::new(child),
        None,
    )
    .map_err(|err| RegistryError {
        message: err.to_string(),
    })
}

fn list_u16(values: &[u16; 103]) -> Result<FixedSizeListArray, RegistryError> {
    let child = UInt16Array::from(values.to_vec());
    FixedSizeListArray::try_new(
        Arc::new(Field::new(
            "item",
            lancedb::arrow::arrow_schema::DataType::UInt16,
            false,
        )),
        i32::from(DEFAULT_QUORUM_K),
        Arc::new(child),
        None,
    )
    .map_err(|err| RegistryError {
        message: err.to_string(),
    })
}

fn mask_array(mask: &NoiseBitmask1024) -> Result<FixedSizeBinaryArray, RegistryError> {
    let mut bytes = Vec::with_capacity(128);
    for word in mask.words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    FixedSizeBinaryArray::try_new(128, Buffer::from(bytes), None).map_err(|err| RegistryError {
        message: err.to_string(),
    })
}

fn append_batch(
    batch: &RecordBatch,
    locks_by_id: &mut HashMap<String, Arc<DomainHypercubeLock>>,
    all_locks: &mut Vec<Arc<DomainHypercubeLock>>,
) -> Result<(), RegistryError> {
    let domain_ids = col_string(batch, "domain_id")?;
    let versions = col_u64(batch, "version")?;
    let polarities = col_u8(batch, "polarity")?;
    let l1 = col_f32(batch, "l1_cosine_threshold")?;
    let l2 = col_f32(batch, "l2_clause_threshold")?;
    let epsilons = col_f32(batch, "epsilon")?;
    let quorums = col_u16(batch, "quorum_k")?;
    let centroids = col_fsl(batch, "centroid_vector")?;
    let los = col_fsl(batch, "lo_vector")?;
    let his = col_fsl(batch, "hi_vector")?;
    let topk = col_fsl(batch, "top_k_indices")?;
    let masks = col_bin(batch, "noise_mask")?;
    let centroid_vals = f32_child(centroids)?;
    let lo_vals = f32_child(los)?;
    let hi_vals = f32_child(his)?;
    let topk_vals = u16_child(topk)?;

    for row in 0..batch.num_rows() {
        let centroid = row_f32(centroid_vals, row)?;
        let lo = row_f32(lo_vals, row)?;
        let hi = row_f32(hi_vals, row)?;
        let epsilon = epsilons.value(row);
        let mut words = [0u64; NOISE_MASK_WORDS];
        let mask_bytes = masks.value(row);
        if mask_bytes.len() != 128 {
            return Err(RegistryError {
                message: format!("noise_mask width {}", mask_bytes.len()),
            });
        }
        for (idx, word) in words.iter_mut().enumerate() {
            let start = idx * 8;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&mask_bytes[start..start + 8]);
            *word = u64::from_le_bytes(buf);
        }
        let k = usize::from(DEFAULT_QUORUM_K);
        let mut top_k_indices = [0u16; MAX_TOPK];
        for (slot, index) in top_k_indices.iter_mut().take(k).enumerate() {
            *index = topk_vals.value(row * k + slot);
        }
        let domain_id = domain_ids.value(row).to_string();
        let lock = Arc::new(DomainHypercubeLock::from_spec(LockSpec {
            domain_id: domain_id.clone(),
            centroid,
            lo,
            hi,
            epsilon,
            noise_mask: NoiseBitmask1024 { words },
            top_k_indices,
            top_k_len: DEFAULT_QUORUM_K,
            quorum_k: quorums.value(row),
            l1_cosine_threshold: l1.value(row),
            l2_clause_threshold: l2.value(row),
            polarity: polarities.value(row),
            domain_version: versions.value(row),
        }));
        locks_by_id.insert(domain_id, Arc::clone(&lock));
        all_locks.push(lock);
    }
    Ok(())
}

fn row_f32(values: &Float32Array, row: usize) -> Result<AlignedVec1024, RegistryError> {
    let start = row * DIM;
    let slice = &values.values()[start..start + DIM];
    load_validated_vec1024(slice).map_err(RegistryError::from)
}

fn col_string<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a StringArray, RegistryError> {
    batch
        .column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| RegistryError {
            message: format!("missing {name}"),
        })
}

fn col_u64<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a UInt64Array, RegistryError> {
    downcast(batch, name)
}

fn col_u8<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a UInt8Array, RegistryError> {
    downcast(batch, name)
}

fn col_u16<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a UInt16Array, RegistryError> {
    downcast(batch, name)
}

fn col_f32<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a Float32Array, RegistryError> {
    downcast(batch, name)
}

fn col_fsl<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a FixedSizeListArray, RegistryError> {
    downcast(batch, name)
}

fn col_bin<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a FixedSizeBinaryArray, RegistryError> {
    downcast(batch, name)
}

fn downcast<'a, T: 'static>(batch: &'a RecordBatch, name: &str) -> Result<&'a T, RegistryError> {
    batch
        .column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<T>())
        .ok_or_else(|| RegistryError {
            message: format!("missing {name}"),
        })
}

fn f32_child(list: &FixedSizeListArray) -> Result<&Float32Array, RegistryError> {
    list.values()
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| RegistryError {
            message: "fixed-size list child is not Float32".to_string(),
        })
}

fn u16_child(list: &FixedSizeListArray) -> Result<&UInt16Array, RegistryError> {
    list.values()
        .as_any()
        .downcast_ref::<UInt16Array>()
        .ok_or_else(|| RegistryError {
            message: "fixed-size list child is not UInt16".to_string(),
        })
}
