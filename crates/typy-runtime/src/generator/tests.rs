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
