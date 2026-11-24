//! Tests of the stack check of recursive functions.

use super::{record_limit, typy_rt_check_stack};

#[test]
#[cfg_attr(
    miri,
    ignore = "Miri cannot run the inline assembly that reads the stack pointer"
)]
fn a_stack_past_its_limit_raises_recursion_error() {
    super::LIMIT.with(|limit| *limit = usize::MAX);
    let exception = typy_rt_check_stack();
    super::LIMIT.with(|limit| *limit = 0);
    assert!(!exception.is_null());
    // SAFETY: the exception was just raised and nothing else refers to it.
    unsafe {
        assert_eq!((*exception).kind(), crate::ExceptionKind::RecursionError);
        crate::refcount::typy_rt_decrement(exception.cast());
    }
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot call `getrlimit`")]
fn a_shallow_stack_passes_the_check() {
    record_limit();
    assert!(typy_rt_check_stack().is_null());
}

#[test]
#[cfg_attr(miri, ignore = "Miri cannot read a thread's stack")]
fn the_limit_lies_within_a_small_threads_own_stack() {
    const SIZE: usize = 256 * 1024;
    let (stack, limit, again) = std::thread::Builder::new()
        .stack_size(SIZE)
        .spawn(|| {
            record_limit();
            let limit = super::LIMIT.with(|limit| *limit);
            // A second export call on the thread finds the limit it kept.
            record_limit();
            (
                super::stack_pointer(),
                limit,
                super::LIMIT.with(|limit| *limit),
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(limit < stack, "{limit:#x} {stack:#x}");
    assert!(stack - limit < SIZE, "{limit:#x} {stack:#x}");
    assert_eq!(again, limit);
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(miri, ignore = "Miri cannot call `getrlimit`")]
fn the_main_threads_limit_is_rlimit_stack_below_the_stack_pointer() {
    super::record_limit_on(true);
    let (stack, limit) = (super::stack_pointer(), super::LIMIT.with(|limit| *limit));
    let size = usize::try_from(crate::libc::stack_limit().unwrap_or(super::DEFAULT_STACK)).unwrap();
    // The limit was taken a few frames deeper than `stack`, so it lies at least that far
    // below it.
    assert!(
        limit <= stack.saturating_sub(size.saturating_sub(super::MARGIN)),
        "{limit:#x} {stack:#x}"
    );
    assert!(typy_rt_check_stack().is_null());
}
