//! Opt-in deterministic fault seam for the portable safety oracle.
//! No production build enables this feature by default.
use std::{
    cell::RefCell,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
};

thread_local! {
    static READ_FAILURE: RefCell<Option<(PathBuf, usize)>> = const { RefCell::new(None) };
}

/// Owns one thread-local, path-specific read failure. Dropping also resets an
/// unconsumed fault, so independent tests and later operations cannot inherit it.
pub struct SourceReadFailureGuard(PhantomData<Rc<()>>);

pub fn fail_source_observation_after_reads(
    path: &Path,
    successful_reads: usize,
) -> SourceReadFailureGuard {
    READ_FAILURE.with(|failure| {
        assert!(failure.borrow().is_none(), "nested fault arrangement");
        *failure.borrow_mut() = Some((path.to_owned(), successful_reads));
    });
    SourceReadFailureGuard(PhantomData)
}

impl Drop for SourceReadFailureGuard {
    fn drop(&mut self) {
        READ_FAILURE.with(|failure| *failure.borrow_mut() = None);
    }
}

pub(crate) fn source_observation_fails(path: &Path) -> bool {
    READ_FAILURE.with(|failure| {
        let mut failure = failure.borrow_mut();
        let Some((target, remaining)) = failure.as_mut() else {
            return false;
        };
        if target != path {
            return false;
        }
        if *remaining > 0 {
            *remaining -= 1;
            return false;
        }
        *failure = None;
        true
    })
}
