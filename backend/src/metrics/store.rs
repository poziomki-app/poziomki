use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Number of slots in each ring buffer.
/// At 10-second intervals: 8640 slots = 24 hours of data.
pub const RING_CAPACITY: usize = 8640;

/// A lock-free ring buffer storing `(timestamp, value)` pairs.
///
/// Single-writer (the sampler task), multiple-reader (API handlers).
/// Each slot stores a `u32` epoch timestamp and an `f32` value (bit-cast to `u32`).
pub struct TimeSeries {
    timestamps: Box<[AtomicU32]>,
    values: Box<[AtomicU32]>,
    head: AtomicUsize,
    count: AtomicUsize,
}

impl TimeSeries {
    pub fn new() -> Self {
        let timestamps: Vec<AtomicU32> = (0..RING_CAPACITY).map(|_| AtomicU32::new(0)).collect();
        let values: Vec<AtomicU32> = (0..RING_CAPACITY).map(|_| AtomicU32::new(0)).collect();
        Self {
            timestamps: timestamps.into_boxed_slice(),
            values: values.into_boxed_slice(),
            head: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    /// Push a new sample. Single-writer only (called from sampler task).
    pub fn push(&self, timestamp_epoch: u32, value: f32) {
        let head = self.head.load(Ordering::Relaxed);
        if let (Some(ts_slot), Some(val_slot)) =
            (self.timestamps.get(head), self.values.get(head))
        {
            ts_slot.store(timestamp_epoch, Ordering::Relaxed);
            val_slot.store(value.to_bits(), Ordering::Release);
        }
        self.head
            .store((head + 1) % RING_CAPACITY, Ordering::Relaxed);
        let prev_count = self.count.load(Ordering::Relaxed);
        if prev_count < RING_CAPACITY {
            self.count.store(prev_count + 1, Ordering::Relaxed);
        }
    }

    /// Read all samples whose timestamp falls within `[from, to]`.
    /// Returns `(timestamps_vec, values_vec)` in chronological order.
    pub fn read_range(&self, from: u32, to: u32) -> (Vec<u32>, Vec<f32>) {
        let count = self.count.load(Ordering::Relaxed);
        if count == 0 {
            return (Vec::new(), Vec::new());
        }
        let head = self.head.load(Ordering::Relaxed);

        // The oldest entry is at (head - count) mod RING_CAPACITY
        let start = if head >= count {
            head - count
        } else {
            RING_CAPACITY - (count - head)
        };

        let mut timestamps = Vec::with_capacity(count);
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let idx = (start + i) % RING_CAPACITY;
            let (Some(ts_slot), Some(val_slot)) =
                (self.timestamps.get(idx), self.values.get(idx))
            else {
                continue;
            };
            let ts = ts_slot.load(Ordering::Relaxed);
            if ts >= from && ts <= to {
                let val_bits = val_slot.load(Ordering::Acquire);
                timestamps.push(ts);
                values.push(f32::from_bits(val_bits));
            }
        }

        (timestamps, values)
    }
}

impl Default for TimeSeries {
    fn default() -> Self {
        Self::new()
    }
}
