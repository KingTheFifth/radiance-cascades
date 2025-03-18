use std::sync::{atomic::Ordering, LazyLock};
use std::time::Instant;

use atomic_float::AtomicF32;

static START: LazyLock<Instant> = LazyLock::new(Instant::now);
// Initialize with 1.0 because 0.0 is scary.
static DELTA: LazyLock<AtomicF32> = LazyLock::new(|| AtomicF32::new(1.0));

pub(crate) fn initialize() {
    let _ = &*START;
}

pub fn elapsed_time() -> f32 {
    Instant::now().duration_since(*START).as_secs_f32()
}

pub(crate) fn set_delta_time(f: f32) {
    DELTA.store(f, Ordering::Release)
}

pub fn delta_time() -> f32 {
    DELTA.load(Ordering::Acquire)
}
