use super::delegate::typy_rt_generator_delegate;
use super::result::{
    typy_rt_generator_has_result, typy_rt_generator_result, typy_rt_generator_set_result,
};
use super::{
    FINISHED, GeneratorDescriptor, GeneratorObject, NEW, PREFIX, SUSPENDED, advance, current,
    generator_repr, typy_rt_generator_close, typy_rt_generator_close_dropped,
    typy_rt_generator_new, typy_rt_generator_send, typy_rt_generator_throw,
};
use crate::exception::{
    ExceptionKind, ExceptionObject, handled_exception, raise, raise_bare, typy_rt_exception_enter,
    typy_rt_exception_leave,
};
use crate::list::ItemKind;
use crate::string::{StrObject, from_str};
use crate::tuple::{Field, TupleType};

fn text(string: *mut StrObject) -> String {
    // SAFETY: the string is live.
    unsafe {
        (0..crate::string::length(string))
            .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
            .collect()
    }
}

/// Returns the `int` word of `value`.
fn word(value: i64) -> u64 {
    crate::word::inline(value).unwrap().cast_unsigned()
}

/// A body that yields 10, then 20, then ends, as compiled resume code steps.
unsafe extern "C" fn two_items(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    match object.point {
        0 | 1 => {
            object.current = word(10 * i64::from(object.point + 1));
            object.point += 1;
            object.state = SUSPENDED;
        }
        _ => {}
    }
    core::ptr::null_mut()
}

/// A body that steps its own generator, which is running.
unsafe extern "C" fn reentering(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the generator is live.
    let inner = unsafe { advance(generator, word(1).cast_signed()) };
    inner.exception
}

/// A body that raises `StopIteration`.
unsafe extern "C" fn stopping(_: *mut GeneratorObject) -> *mut ExceptionObject {
    crate::exception::raise_bare(ExceptionKind::StopIteration)
}

/// Returns a descriptor of `step` whose frame holds one object and one `int`, the object
/// an argument, and whose items are `int`s.
fn descriptor(
    step: unsafe extern "C" fn(*mut GeneratorObject) -> *mut ExceptionObject,
    frame: &TupleType,
) -> GeneratorDescriptor {
    GeneratorDescriptor {
        name: from_str("count"),
        qualified_name: from_str("outer.<locals>.count"),
        step,
        frame,
        arguments: 1,
        item_kind: ItemKind::Int as u64,
        item_tuple: core::ptr::null(),
        result: core::ptr::null(),
    }
}

fn release_names(descriptor: &GeneratorDescriptor) {
    for each in [descriptor.name, descriptor.qualified_name] {
        // SAFETY: the test made each name and holds its reference.
        unsafe { crate::refcount::typy_rt_decrement(each.cast_mut().cast()) };
    }
}

fn frame_layout(fields: &[Field; 2]) -> TupleType {
    TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }
}

const FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Object, 0),
    Field::plain(ItemKind::Int, 8),
];

#[test]
fn smt_11_a_generator_yields_each_item_then_stops() {
    let fields = FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(two_items, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let generator = typy_rt_generator_new(&raw const found);
        assert_eq!((*generator).state, NEW);
        let mut items = Vec::new();
        while advance(generator, word(0).cast_signed()).value {
            items.push(current(generator));
        }
        assert_eq!(items, [word(10), word(20)]);
        assert_eq!((*generator).state, FINISHED);
        // A finished generator stays exhausted, and `next()` of it raises `StopIteration`.
        assert!(!advance(generator, word(0).cast_signed()).value);
        let stopped = advance(generator, word(1).cast_signed());
        assert_eq!((*stopped.exception).kind(), ExceptionKind::StopIteration);
        crate::refcount::typy_rt_decrement(stopped.exception.cast());
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
}

#[test]
fn smt_11_a_running_generator_and_an_escaping_stop_iteration_raise_as_cpython_does() {
    let fields = FIELDS;
    let layout = frame_layout(&fields);
    for (step, kind, message) in [
        (
            reentering as unsafe extern "C" fn(*mut GeneratorObject) -> *mut ExceptionObject,
            ExceptionKind::ValueError,
            "generator already executing",
        ),
        (
            stopping,
            ExceptionKind::RuntimeError,
            "generator raised StopIteration",
        ),
    ] {
        let found = descriptor(step, &layout);
        // SAFETY: the descriptor and its layout live for the whole loop.
        unsafe {
            let generator = typy_rt_generator_new(&raw const found);
            let raised = advance(generator, word(1).cast_signed()).exception;
            assert_eq!((*raised).kind(), kind);
            assert_eq!(text((*raised).message), message);
            if kind == ExceptionKind::RuntimeError {
                assert_eq!((*(*raised).cause).kind(), ExceptionKind::StopIteration);
                assert_eq!((*raised).suppress_context, 1);
            }
            assert_eq!((*generator).state, FINISHED);
            crate::refcount::typy_rt_decrement(raised.cast());
            crate::refcount::typy_rt_decrement(generator.cast());
        }
        release_names(&found);
    }
}

#[test]
fn smt_11_a_freed_generator_releases_its_arguments_and_frame_unless_it_finished() {
    let fields = FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(two_items, &layout);
    let held = from_str("argument");
    // SAFETY: the descriptor, its layout and the string live for the whole test.
    unsafe {
        for finished in [false, true] {
            let generator = typy_rt_generator_new(&raw const found);
            let frame = core::ptr::with_exposed_provenance_mut::<u64>(
                generator.expose_provenance() + PREFIX,
            );
            // The frame takes over a reference to its argument, as `SetGeneratorArgument`
            // does.
            crate::refcount::typy_rt_increment(held.cast());
            *frame = held.expose_provenance() as u64;
            if finished {
                (*generator).state = FINISHED;
            }
            assert_eq!((*held).header.reference_count, 2);
            crate::refcount::typy_rt_decrement(generator.cast());
            assert_eq!((*held).header.reference_count, 1);
        }
        crate::refcount::typy_rt_decrement(held.cast());
    }
    release_names(&found);
}

#[test]
fn smt_11_a_generator_writes_its_qualified_name_and_address() {
    let fields = FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(two_items, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let generator = typy_rt_generator_new(&raw const found);
        let object = generator.cast();
        let repr = generator_repr(object).value;
        let address = object.expose_provenance();
        assert_eq!(
            text(repr),
            format!("<generator object outer.<locals>.count at {address:#x}>")
        );
        let ty = crate::type_object::type_of(object);
        let class = crate::type_object::typy_rt_type_text(ty);
        assert_eq!(text(class), "<class 'generator'>");
        // `iter()` of a generator is the generator.
        let iterator = crate::iterator::typy_rt_iterable_iterator(object, core::ptr::null());
        assert_eq!(iterator.cast::<crate::header::ObjectHeader>(), object);
        crate::refcount::typy_rt_decrement(iterator.cast());
        for each in [repr, class] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
}

/// A body that yields 0, then each value `send` gives, and ends when `next()` resumes it.
unsafe extern "C" fn echoing(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    let item = match (object.point, object.received) {
        (0, _) => word(0),
        (_, 1) => object.sent,
        _ => return core::ptr::null_mut(),
    };
    object.current = item;
    object.point = 1;
    object.state = SUSPENDED;
    core::ptr::null_mut()
}

/// A body that yields 1, then, at its `yield`, raises what `throw` or `close` gave it, or,
/// for a `KeyError`, drops it and yields 2, or, for `GeneratorExit`, raises `ValueError`
/// when its argument says so.
unsafe extern "C" fn catching(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    if object.point == 0 {
        object.current = word(1);
        object.point = 1;
        object.state = SUSPENDED;
        return core::ptr::null_mut();
    }
    let thrown = core::mem::replace(&mut object.thrown, core::ptr::null_mut());
    if thrown.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: the generator handed over the exception it held.
    let kind = unsafe { (*thrown).kind() };
    let frame = core::ptr::with_exposed_provenance::<u64>(generator.expose_provenance() + PREFIX);
    // SAFETY: the frame's first field is the argument the test stored.
    let replace = unsafe { *frame } != 0;
    match kind {
        ExceptionKind::KeyError => {
            // SAFETY: the body owns the exception it drops.
            unsafe { crate::refcount::typy_rt_decrement(thrown.cast()) };
            object.current = word(2);
            object.state = SUSPENDED;
            core::ptr::null_mut()
        }
        ExceptionKind::GeneratorExit if replace => {
            // SAFETY: the body owns the exception it replaces.
            unsafe { crate::refcount::typy_rt_decrement(thrown.cast()) };
            raise(ExceptionKind::ValueError, "replaced")
        }
        _ => thrown,
    }
}

/// A body that calls `send`, `throw` or `close`, as its point says, of its own generator.
unsafe extern "C" fn stepping_itself(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the generator is live, and its step is running.
    unsafe {
        match (*generator).point {
            0 => typy_rt_generator_send(generator, 0, true).exception,
            1 => typy_rt_generator_throw(generator, raise_bare(ExceptionKind::KeyError)).exception,
            _ => typy_rt_generator_close(generator),
        }
    }
}

/// A body that handles an exception of its own while it yields, as a `yield` inside an
/// `except` clause does, and yields the exception its caller was handling as an item.
unsafe extern "C" fn handling(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    let own = raise_bare(ExceptionKind::IndexError);
    // SAFETY: the body keeps the exception alive while it is handled, then drops it.
    unsafe {
        typy_rt_exception_enter(own);
        object.current = object.caller.expose_provenance() as u64;
        object.state = SUSPENDED;
        crate::refcount::typy_rt_decrement(own.cast());
    }
    core::ptr::null_mut()
}

/// Returns a generator of `found`, with its argument set to `argument`.
///
/// # Safety
///
/// `found` lives as long as the generator.
unsafe fn generator_of(found: &GeneratorDescriptor, argument: u64) -> *mut GeneratorObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let generator = typy_rt_generator_new(found);
        let frame =
            core::ptr::with_exposed_provenance_mut::<u64>(generator.expose_provenance() + PREFIX);
        *frame = argument;
        generator
    }
}

/// The frame of the bodies whose argument is a flag, not an object.
const FLAG_FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Int, 8),
];

/// Releases a raised exception after checking its kind and message.
///
/// # Safety
///
/// `raised` is a live exception the test owns.
unsafe fn expect_raised(raised: *mut ExceptionObject, kind: ExceptionKind, message: Option<&str>) {
    assert!(!raised.is_null(), "expected {kind:?}");
    // SAFETY: the caller's guarantee.
    unsafe {
        assert_eq!((*raised).kind(), kind);
        if let Some(message) = message {
            assert_eq!(text((*raised).message), message);
        }
        crate::refcount::typy_rt_decrement(raised.cast());
    }
}

#[test]
fn fun_07_send_gives_its_value_to_the_step_and_returns_the_next_item() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(echoing, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let generator = generator_of(&found, 0);
        let refused = typy_rt_generator_send(generator, word(5), false);
        expect_raised(
            refused.exception,
            ExceptionKind::TypeError,
            Some("can't send non-None value to a just-started generator"),
        );
        assert_eq!((*generator).state, NEW);
        let started = typy_rt_generator_send(generator, 0, true);
        assert!(started.exception.is_null() && started.value);
        assert_eq!(current(generator), word(0));
        let echoed = typy_rt_generator_send(generator, word(5), false);
        assert!(echoed.exception.is_null());
        assert_eq!(current(generator), word(5));
        // `next()` resumes it with nothing received, so its body ends.
        let ended = advance(generator, word(1).cast_signed());
        expect_raised(ended.exception, ExceptionKind::StopIteration, None);
        let finished = typy_rt_generator_send(generator, word(6), false);
        expect_raised(finished.exception, ExceptionKind::StopIteration, None);
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
}

#[test]
fn fun_07_throw_raises_at_the_yield_and_finishes_a_generator_not_started() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(catching, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let unstarted = generator_of(&found, 0);
        let thrown = raise_bare(ExceptionKind::ValueError);
        let raised = typy_rt_generator_throw(unstarted, thrown);
        assert_eq!(raised.exception, thrown);
        assert_eq!((*unstarted).state, FINISHED);
        expect_raised(raised.exception, ExceptionKind::ValueError, None);
        crate::refcount::typy_rt_decrement(unstarted.cast());
        let generator = generator_of(&found, 0);
        assert!(advance(generator, word(1).cast_signed()).value);
        // The body drops a `KeyError` and yields again.
        let caught = typy_rt_generator_throw(generator, raise_bare(ExceptionKind::KeyError));
        assert!(caught.exception.is_null() && caught.value);
        assert_eq!(current(generator), word(2));
        let escaped = typy_rt_generator_throw(generator, raise_bare(ExceptionKind::IndexError));
        expect_raised(escaped.exception, ExceptionKind::IndexError, None);
        assert_eq!((*generator).state, FINISHED);
        let again = typy_rt_generator_throw(generator, raise_bare(ExceptionKind::KeyError));
        expect_raised(again.exception, ExceptionKind::KeyError, None);
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
}

#[test]
fn fun_11_close_raises_generator_exit_and_refuses_a_body_that_yields_again() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(catching, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let unstarted = generator_of(&found, 0);
        assert!(typy_rt_generator_close(unstarted).is_null());
        assert_eq!((*unstarted).state, FINISHED);
        assert!(typy_rt_generator_close(unstarted).is_null());
        crate::refcount::typy_rt_decrement(unstarted.cast());
        // A body that raises the `GeneratorExit` it was given closes quietly.
        let quiet = generator_of(&found, 0);
        assert!(advance(quiet, word(1).cast_signed()).value);
        assert!(typy_rt_generator_close(quiet).is_null());
        assert_eq!((*quiet).state, FINISHED);
        crate::refcount::typy_rt_decrement(quiet.cast());
        // A body that raises another exception passes it on.
        let replacing = generator_of(&found, 1);
        assert!(advance(replacing, word(1).cast_signed()).value);
        expect_raised(
            typy_rt_generator_close(replacing),
            ExceptionKind::ValueError,
            Some("replaced"),
        );
        crate::refcount::typy_rt_decrement(replacing.cast());
    }
    release_names(&found);
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(two_items, &layout);
    // SAFETY: the descriptor and its layout live for the rest of the test.
    unsafe {
        // `two_items` ignores what it is given and yields again.
        let stubborn = generator_of(&found, 0);
        assert!(advance(stubborn, word(1).cast_signed()).value);
        expect_raised(
            typy_rt_generator_close(stubborn),
            ExceptionKind::RuntimeError,
            Some("generator ignored GeneratorExit"),
        );
        // It is still suspended, so freeing it would close it again; the test frees it as
        // one that closing left suspended.
        assert_eq!((*stubborn).state, SUSPENDED);
        (*stubborn).state = super::ABANDONED;
        crate::refcount::typy_rt_decrement(stubborn.cast());
    }
    release_names(&found);
}

#[test]
fn fun_07_a_running_generator_refuses_send_throw_and_close() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(stepping_itself, &layout);
    for point in 0..3 {
        // SAFETY: the descriptor and its layout live for the whole loop.
        unsafe {
            let generator = generator_of(&found, 0);
            (*generator).point = point;
            let raised = advance(generator, word(1).cast_signed()).exception;
            expect_raised(
                raised,
                ExceptionKind::ValueError,
                Some("generator already executing"),
            );
            crate::refcount::typy_rt_decrement(generator.cast());
        }
    }
    release_names(&found);
}

#[test]
fn fun_07_a_step_hands_back_the_exception_its_caller_was_handling() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(handling, &layout);
    let caller = raise_bare(ExceptionKind::KeyError);
    // SAFETY: the descriptor, its layout and the caller's exception live for the whole test.
    unsafe {
        let previous = typy_rt_exception_enter(caller);
        let generator = generator_of(&found, 0);
        assert!(advance(generator, word(1).cast_signed()).value);
        // The body saw the caller's exception, and the caller handles it again.
        assert_eq!(current(generator), caller.expose_provenance() as u64);
        assert_eq!(handled_exception(), caller);
        (*generator).current = 0;
        // The body holds nothing more, so the test frees it as one that ended.
        (*generator).state = FINISHED;
        crate::refcount::typy_rt_decrement(generator.cast());
        typy_rt_exception_leave(previous);
        crate::refcount::typy_rt_decrement(caller.cast());
    }
    release_names(&found);
}

std::thread_local! {
    /// The flags of the generators `recording` closed, in order.
    static CLOSED: core::cell::RefCell<Vec<u64>> = const { core::cell::RefCell::new(Vec::new()) };
}

/// A body that yields, then, when it is closed, records its argument, an `int` word, and
/// ends; one whose argument is the word of 0 yields again instead, as a body that ignores
/// `GeneratorExit` does.
unsafe extern "C" fn recording(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    let frame = core::ptr::with_exposed_provenance::<u64>(generator.expose_provenance() + PREFIX);
    // SAFETY: the frame's first field is the argument the test stored.
    let argument = unsafe { *frame };
    if object.point == 0 || argument == word(0) {
        object.current = word(1);
        object.point = 1;
        object.state = SUSPENDED;
        let thrown = core::mem::replace(&mut object.thrown, core::ptr::null_mut());
        if !thrown.is_null() {
            // SAFETY: the body owns the exception it ignores.
            unsafe { crate::refcount::typy_rt_decrement(thrown.cast()) };
        }
        return core::ptr::null_mut();
    }
    CLOSED.with(|closed| closed.borrow_mut().push(argument));
    // The body ends by raising the `GeneratorExit` it was given.
    core::mem::replace(&mut object.thrown, core::ptr::null_mut())
}

#[test]
fn fun_11_a_generator_freed_while_suspended_waits_then_closes_in_the_order_freed() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(recording, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let unstarted = generator_of(&found, word(7));
        crate::refcount::typy_rt_decrement(unstarted.cast());
        let generators: Vec<*mut GeneratorObject> = [3, 1, 2]
            .map(|argument| generator_of(&found, word(argument)))
            .to_vec();
        for each in &generators {
            assert!(advance(*each, word(1).cast_signed()).value);
        }
        for each in generators {
            crate::refcount::typy_rt_decrement(each.cast());
        }
        // Freeing runs no code of the body: the generators wait in the queue, and the one
        // never started was freed at once.
        assert!(CLOSED.with(|closed| closed.borrow().is_empty()));
        // Making a generator installed the closing the end of the program runs.
        super::close_dropped_at_exit();
        assert_eq!(
            CLOSED.with(|closed| closed.borrow().clone()),
            [word(3), word(1), word(2)]
        );
        // One that yields again is reported and freed without closing it again.
        let stubborn = generator_of(&found, word(0));
        assert!(advance(stubborn, word(1).cast_signed()).value);
        crate::refcount::typy_rt_decrement(stubborn.cast());
        typy_rt_generator_close_dropped();
        assert!(super::dropped::take().is_null());
        assert_eq!(CLOSED.with(|closed| closed.borrow().len()), 3);
    }
    release_names(&found);
}

/// The layout of a one-item tuple of an `int`, the result of the bodies below.
const RESULT_FIELDS: [Field; 1] = [Field::plain(ItemKind::Int, 0)];

/// A body that yields once, then returns the `int` its argument is, keeping it as its
/// result as compiled `return value` does.
unsafe extern "C" fn returning(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    if object.point == 0 {
        object.current = word(1);
        object.point = 1;
        object.state = SUSPENDED;
        return core::ptr::null_mut();
    }
    let thrown = core::mem::replace(&mut object.thrown, core::ptr::null_mut());
    if !thrown.is_null() {
        return thrown;
    }
    let frame = core::ptr::with_exposed_provenance::<u64>(generator.expose_provenance() + PREFIX);
    // SAFETY: the frame's first field is the argument the test stored, and the body runs.
    unsafe { typy_rt_generator_set_result(generator, *frame) };
    core::ptr::null_mut()
}

/// Returns the descriptor of `returning`, whose result is laid out as `result`.
fn returning_descriptor(frame: &TupleType, result: &TupleType) -> GeneratorDescriptor {
    GeneratorDescriptor {
        result,
        ..descriptor(returning, frame)
    }
}

#[test]
fn fun_07_a_returned_value_is_the_argument_of_stop_iteration_or_the_value_of_yield_from() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let result_fields = RESULT_FIELDS;
    let result = TupleType {
        size: 8,
        length: 1,
        fields: result_fields.as_ptr(),
    };
    let found = returning_descriptor(&layout, &result);
    // SAFETY: the descriptors and layouts live for the whole test.
    unsafe {
        let stopped = generator_of(&found, word(7));
        assert!(advance(stopped, word(1).cast_signed()).value);
        let raised = advance(stopped, word(1).cast_signed()).exception;
        assert_eq!((*raised).kind(), ExceptionKind::StopIteration);
        let message = crate::exception::typy_rt_exception_str(raised.cast()).value;
        assert_eq!(text(message), "7");
        crate::refcount::typy_rt_decrement(message.cast());
        crate::refcount::typy_rt_decrement(raised.cast());
        assert!(!typy_rt_generator_has_result(stopped.cast()));
        crate::refcount::typy_rt_decrement(stopped.cast());
        // A delegation keeps the result for `yield from` to read.
        let delegator = generator_of(&found, word(0));
        let delegated = generator_of(&found, word(9));
        assert!(typy_rt_generator_delegate(delegator, delegated.cast(), true).value);
        let ended = typy_rt_generator_delegate(delegator, delegated.cast(), false);
        assert!(ended.exception.is_null() && !ended.value);
        assert!(typy_rt_generator_has_result(delegated.cast()));
        assert_eq!(typy_rt_generator_result(delegated.cast()).value, word(9));
        crate::refcount::typy_rt_decrement(delegated.cast());
        crate::refcount::typy_rt_decrement(delegator.cast());
    }
    release_names(&found);
}

#[test]
fn smt_11_yield_from_passes_on_what_resumes_the_generator() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(echoing, &layout);
    let catching_found = descriptor(catching, &layout);
    // SAFETY: the descriptors and layouts live for the whole test.
    unsafe {
        let delegator = generator_of(&found, 0);
        let echo = generator_of(&found, 0);
        assert!(typy_rt_generator_delegate(delegator, echo.cast(), true).value);
        // A value `send` gave the delegator is sent on.
        (*delegator).received = super::VALUE;
        (*delegator).sent = word(4);
        assert!(typy_rt_generator_delegate(delegator, echo.cast(), false).value);
        assert_eq!(current(echo), word(4));
        // `next()` ends `echoing`.
        (*delegator).received = 0;
        assert!(!typy_rt_generator_delegate(delegator, echo.cast(), false).value);
        crate::refcount::typy_rt_decrement(echo.cast());
        // `GeneratorExit` closes the generator delegated to, then leaves.
        let caught = generator_of(&catching_found, 0);
        assert!(typy_rt_generator_delegate(delegator, caught.cast(), true).value);
        (*delegator).thrown = raise_bare(ExceptionKind::GeneratorExit);
        let left = typy_rt_generator_delegate(delegator, caught.cast(), false);
        expect_raised(left.exception, ExceptionKind::GeneratorExit, None);
        assert_eq!((*caught).state, FINISHED);
        crate::refcount::typy_rt_decrement(caught.cast());
        // Any other exception is thrown in.
        let thrown_into = generator_of(&catching_found, 0);
        assert!(typy_rt_generator_delegate(delegator, thrown_into.cast(), true).value);
        (*delegator).thrown = raise_bare(ExceptionKind::KeyError);
        assert!(typy_rt_generator_delegate(delegator, thrown_into.cast(), false).value);
        assert_eq!(current(thrown_into), word(2));
        (*thrown_into).state = FINISHED;
        crate::refcount::typy_rt_decrement(thrown_into.cast());
        crate::refcount::typy_rt_decrement(delegator.cast());
    }
    release_names(&found);
    release_names(&catching_found);
}

#[test]
fn smt_11_an_iterator_that_is_no_generator_takes_next_alone() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(echoing, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let delegator = generator_of(&found, 0);
        let list = crate::list::typy_rt_list_new(
            word(ItemKind::Int as i64).cast_signed(),
            word(1).cast_signed(),
            0,
        );
        crate::list::typy_rt_list_append(list, word(5));
        let iterator = crate::iterator::typy_rt_iterable_iterator(list.cast(), core::ptr::null());
        assert!(typy_rt_generator_delegate(delegator, iterator.cast(), true).value);
        (*delegator).received = super::VALUE;
        (*delegator).sent = word(1);
        let refused = typy_rt_generator_delegate(delegator, iterator.cast(), false);
        expect_raised(
            refused.exception,
            ExceptionKind::AttributeError,
            Some("'list_iterator' object has no attribute 'send'"),
        );
        (*delegator).thrown = raise_bare(ExceptionKind::KeyError);
        let thrown = typy_rt_generator_delegate(delegator, iterator.cast(), false);
        expect_raised(thrown.exception, ExceptionKind::KeyError, None);
        expect_raised(
            typy_rt_generator_result(iterator.cast()).exception,
            ExceptionKind::TypeError,
            None,
        );
        for each in [
            iterator.cast::<crate::header::ObjectHeader>(),
            list.cast(),
            delegator.cast(),
        ] {
            crate::refcount::typy_rt_decrement(each);
        }
    }
    release_names(&found);
}

/// A body that yields once, then, when it is closed or sent a value, drops what it was
/// thrown and returns its argument as its result.
unsafe extern "C" fn quitting(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    if object.point == 0 {
        object.current = word(1);
        object.point = 1;
        object.state = SUSPENDED;
        return core::ptr::null_mut();
    }
    let thrown = core::mem::replace(&mut object.thrown, core::ptr::null_mut());
    let frame = core::ptr::with_exposed_provenance::<u64>(generator.expose_provenance() + PREFIX);
    // SAFETY: the body owns what it was thrown, and its frame holds its argument.
    unsafe {
        if !thrown.is_null() {
            crate::refcount::typy_rt_decrement(thrown.cast());
        }
        typy_rt_generator_set_result(generator, *frame);
    }
    core::ptr::null_mut()
}

/// A body that raises `StopIteration` while it handles a `KeyError`.
unsafe extern "C" fn stopping_while_handling(_: *mut GeneratorObject) -> *mut ExceptionObject {
    let handled = raise_bare(ExceptionKind::KeyError);
    // SAFETY: the body keeps the exception alive while it is handled.
    unsafe {
        typy_rt_exception_enter(handled);
        let stop = raise_bare(ExceptionKind::StopIteration);
        crate::refcount::typy_rt_decrement(handled.cast());
        stop
    }
}

/// A body that yields, then, when it is closed, releases the object its frame holds, a
/// generator of its own, and ends.
unsafe extern "C" fn holding(generator: *mut GeneratorObject) -> *mut ExceptionObject {
    // SAFETY: the runtime passes the live generator whose step is running.
    let object = unsafe { &mut *generator };
    if object.point == 0 {
        object.current = word(1);
        object.point = 1;
        object.state = SUSPENDED;
        return core::ptr::null_mut();
    }
    let frame =
        core::ptr::with_exposed_provenance_mut::<u64>(generator.expose_provenance() + PREFIX);
    // SAFETY: the frame's first field holds the generator the test stored.
    unsafe {
        let held = core::mem::replace(&mut *frame, 0);
        crate::refcount::typy_rt_decrement(core::ptr::with_exposed_provenance_mut::<
            crate::header::ObjectHeader,
        >(usize::try_from(held).unwrap_or(0)));
    }
    core::mem::replace(&mut object.thrown, core::ptr::null_mut())
}

#[test]
fn fun_07_send_and_close_meet_a_body_that_returns() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let result_fields = RESULT_FIELDS;
    let result = TupleType {
        size: 8,
        length: 1,
        fields: result_fields.as_ptr(),
    };
    let found = GeneratorDescriptor {
        result: &raw const result,
        ..descriptor(quitting, &layout)
    };
    // SAFETY: the descriptor and its layouts live for the whole test.
    unsafe {
        let sent = generator_of(&found, word(3));
        assert!(advance(sent, word(1).cast_signed()).value);
        let stopped = typy_rt_generator_send(sent, word(1), false);
        expect_raised(stopped.exception, ExceptionKind::StopIteration, None);
        crate::refcount::typy_rt_decrement(sent.cast());
        let closed = generator_of(&found, word(4));
        assert!(advance(closed, word(1).cast_signed()).value);
        assert!(typy_rt_generator_close(closed).is_null());
        assert_eq!((*closed).state, FINISHED);
        // A finished generator closes at once.
        assert!(typy_rt_generator_close(closed).is_null());
        crate::refcount::typy_rt_decrement(closed.cast());
    }
    release_names(&found);
}

#[test]
fn fun_07_a_result_of_none_is_not_kept_and_one_never_returned_is_missing() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let optional_fields = [Field::plain(ItemKind::OptionalInt, 0)];
    let optional = TupleType {
        size: 8,
        length: 1,
        fields: optional_fields.as_ptr(),
    };
    let none_result = descriptor(two_items, &layout);
    let optional_result = GeneratorDescriptor {
        result: &raw const optional,
        ..descriptor(two_items, &layout)
    };
    // SAFETY: the descriptors and layouts live for the whole test.
    unsafe {
        for found in [&none_result, &optional_result] {
            let generator = generator_of(found, 0);
            typy_rt_generator_set_result(generator, 0);
            assert!(!typy_rt_generator_has_result(generator.cast()));
            expect_raised(
                typy_rt_generator_result(generator.cast()).exception,
                ExceptionKind::TypeError,
                Some("the generator of 'yield from' had ended without returning its value"),
            );
            crate::refcount::typy_rt_decrement(generator.cast());
        }
    }
    release_names(&none_result);
    release_names(&optional_result);
}

#[test]
fn smt_11_an_item_held_inline_is_copied_into_the_generator() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let pair_fields = FLAG_FIELDS;
    let pair = frame_layout(&pair_fields);
    let found = GeneratorDescriptor {
        item_kind: ItemKind::Tuple as u64,
        item_tuple: &raw const pair,
        ..descriptor(two_items, &layout)
    };
    // SAFETY: the descriptor and its layouts live for the whole test.
    unsafe {
        let generator = generator_of(&found, 0);
        for value in [word(5), word(6)] {
            let item = [value, word(7)];
            super::typy_rt_generator_hold(generator, item.as_ptr().expose_provenance() as u64);
            let held = core::ptr::with_exposed_provenance::<u64>(
                usize::try_from(current(generator)).unwrap_or(0),
            );
            assert_eq!(*held, value);
        }
        (*generator).current = 0;
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
    let plain_found = descriptor(two_items, &layout);
    // SAFETY: the descriptor and its layout live for the rest of the test.
    unsafe {
        // The plain kinds keep the item itself.
        let plain = generator_of(&plain_found, 0);
        super::typy_rt_generator_hold(plain, word(8));
        assert_eq!(current(plain), word(8));
        (*plain).current = 0;
        crate::refcount::typy_rt_decrement(plain.cast());
    }
    release_names(&plain_found);
}

#[test]
fn smt_11_a_stop_iteration_raised_while_handling_keeps_its_cause_only() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(stopping_while_handling, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let generator = generator_of(&found, 0);
        let raised = advance(generator, word(1).cast_signed()).exception;
        assert_eq!((*raised).kind(), ExceptionKind::RuntimeError);
        assert!((*raised).context.is_null());
        assert_eq!((*(*raised).cause).kind(), ExceptionKind::StopIteration);
        crate::refcount::typy_rt_decrement(raised.cast());
        crate::refcount::typy_rt_decrement(generator.cast());
    }
    release_names(&found);
}

#[test]
fn fun_11_a_generator_a_closing_drops_closes_before_the_next_one() {
    let flag_fields = FLAG_FIELDS;
    let flag_layout = frame_layout(&flag_fields);
    let inner_found = descriptor(recording, &flag_layout);
    let fields = FIELDS;
    let layout = frame_layout(&fields);
    let outer_found = descriptor(holding, &layout);
    // SAFETY: the descriptors and layouts live for the whole test.
    unsafe {
        let inner = generator_of(&inner_found, word(42));
        assert!(advance(inner, word(1).cast_signed()).value);
        let outer = generator_of(&outer_found, inner.expose_provenance() as u64);
        assert!(advance(outer, word(1).cast_signed()).value);
        let before = CLOSED.with(|closed| closed.borrow().len());
        crate::refcount::typy_rt_decrement(outer.cast());
        typy_rt_generator_close_dropped();
        assert_eq!(
            CLOSED.with(|closed| closed.borrow()[before..].to_vec()),
            [word(42)]
        );
    }
    release_names(&inner_found);
    release_names(&outer_found);
}

#[test]
fn smt_11_yield_from_reports_what_closing_raised_and_ends_at_a_finished_generator() {
    let fields = FLAG_FIELDS;
    let layout = frame_layout(&fields);
    let found = descriptor(catching, &layout);
    // SAFETY: the descriptor and its layout live for the whole test.
    unsafe {
        let delegator = generator_of(&found, 0);
        let replacing = generator_of(&found, 1);
        assert!(typy_rt_generator_delegate(delegator, replacing.cast(), true).value);
        (*delegator).thrown = raise_bare(ExceptionKind::GeneratorExit);
        let raised = typy_rt_generator_delegate(delegator, replacing.cast(), false);
        expect_raised(
            raised.exception,
            ExceptionKind::ValueError,
            Some("replaced"),
        );
        let ended = typy_rt_generator_delegate(delegator, replacing.cast(), true);
        assert!(ended.exception.is_null() && !ended.value);
        crate::refcount::typy_rt_decrement(replacing.cast());
        crate::refcount::typy_rt_decrement(delegator.cast());
    }
    release_names(&found);
}
