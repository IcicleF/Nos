#[cold]
fn cold() {
    std::hint::black_box(())
}

/// Hints to the compiler that the given branch is likely to be taken.
#[inline]
pub fn likely(b: bool) -> bool {
    if !b {
        cold();
    }
    b
}

/// Hints to the compiler that the given branch is unlikely to be taken.
#[inline]
pub fn unlikely(b: bool) -> bool {
    if b {
        cold();
    }
    b
}
