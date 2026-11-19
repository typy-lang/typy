//! Generator objects (SMT-11, FUN-07, ADR-0063): the state of a generator function's body
//! between steps. The object holds its descriptor, its state, the item its last `yield`
//! gave, then its frame, the arguments its call stored and the variables of its body, laid
//! out as the descriptor's tuple type says. A step calls the compiled resume code, which runs
//! the body to its next `yield` or its end. Its `repr` is `<generator object QUALNAME at
//! 0x...>`, and it hashes and compares by identity, as CPython's generators do.

mod dropped;
#[cfg(test)]
mod tests;

use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_bare};
use crate::header::{GENERATOR_TYPE_ID, ObjectHeader};
use crate::list::ItemType;
use crate::string::{StrBuilder, StrObject};
use crate::tuple::TupleType;
use crate::type_object::{OBJECT, TypeObject};

/// What the compiler emits once for each generator function: its names, its resume code,
/// the layout of its frame, and how its items are held.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct GeneratorDescriptor {
    pub(crate) name: *const StrObject,
    pub(crate) qualified_name: *const StrObject,
    /// Runs one step of the body and returns the exception it raised, or null.
    pub(crate) step: unsafe extern "C" fn(*mut GeneratorObject) -> *mut ExceptionObject,
    /// The layout of the frame: the arguments, then the variables of the body.
    pub(crate) frame: *const TupleType,
    /// How many fields of the frame are arguments, which the object releases when it is
    /// freed: the first step takes each but the closure out, leaving zero.
    pub(crate) arguments: u64,
    /// The `ItemKind` code of the items, and the descriptor of a tuple item, or null.
    pub(crate) item_kind: u64,
    pub(crate) item_tuple: *const TupleType,
}

/// A generator object: the header, its descriptor, its state, its current item and what a
/// later step reads, then its frame.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct GeneratorObject {
    header: ObjectHeader,
    descriptor: *const GeneratorDescriptor,
    /// One of the states below, which the runtime sets, and the resume code at a `yield`.
    state: u32,
    /// The `yield` the body stopped at, numbered from one; zero before the first step.
    point: u32,
    /// The item the last `yield` gave, owned until the next step; zero when there is none.
    current: u64,
    /// What `send` passes to the next step.
    sent: u64,
    /// What `throw` raises at the `yield` the next step resumes, owned; null for none.
    thrown: *mut ExceptionObject,
    /// What the body returned, owned until the step that ends reads it.
    result: u64,
    /// Room for an item held inline, a tuple, an optional tuple or a union, whose address is
    /// the current item; null until a `yield` gives one.
    held: *mut u8,
    /// 1 when `send` gave the step that runs a value, which `sent` holds; 0 for `next()`.
    received: u64,
    /// The exception the caller of the step that runs was handling, borrowed, or null: the
    /// outermost `except` clause of the body restores it when it ends.
    caller: *mut ExceptionObject,
    /// The next generator in the queue of those freed while suspended, or null.
    dropped: *mut GeneratorObject,
}

/// The state of a generator whose body has not started.
const NEW: u32 = 0;
/// The state of a generator stopped at a `yield`, which the resume code sets.
const SUSPENDED: u32 = 1;
/// The state of a generator whose step is running.
const RUNNING: u32 = 2;
/// The state of a generator whose body returned or raised.
const FINISHED: u32 = 3;
/// The state of a generator freed while suspended that yielded again when it was closed: it
/// is freed without closing it again, its frame released as a suspended one's.
const ABANDONED: u32 = 4;

/// The bytes before a generator's frame.
pub(crate) const PREFIX: usize = core::mem::size_of::<GeneratorObject>();

/// Returns a new generator of the generator function whose descriptor is `descriptor`, not
/// started, with one reference and every field of its frame zero, which the program then
/// fills with its arguments.
///
/// # Safety
///
/// `descriptor` is a descriptor the compiler emitted, whose frame layout it describes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_new(
    descriptor: *const GeneratorDescriptor,
) -> *mut GeneratorObject {
    // SAFETY: the caller's guarantee.
    let size = unsafe { (*(*descriptor).frame).size };
    let length = PREFIX + usize::try_from(size).unwrap_or(0);
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let generator = crate::allocator::allocate(length).cast::<GeneratorObject>();
    // SAFETY: the allocation holds `length` bytes, aligned for every type.
    unsafe {
        core::ptr::write_bytes(generator.cast::<u8>(), 0, length);
        generator.write(GeneratorObject {
            header: ObjectHeader::new(GENERATOR_TYPE_ID),
            descriptor,
            state: NEW,
            point: 0,
            current: 0,
            sent: 0,
            thrown: core::ptr::null_mut(),
            result: 0,
            held: core::ptr::null_mut(),
            received: 0,
            caller: core::ptr::null_mut(),
            dropped: core::ptr::null_mut(),
        });
    }
    generator
}

/// How a step of a generator ended.
enum Step {
    /// At a `yield`, whose item is the current one.
    Yielded,
    /// At the end of the body, or a `return`.
    Returned,
    /// With this exception, owned; a `StopIteration` already replaced (PEP 479).
    Raised(*mut ExceptionObject),
}

/// Runs one step of `generator`, which is not running and not finished. A generator whose
/// body ends, by returning or raising, is finished after it.
///
/// # Safety
///
/// `generator` points to a live generator that is new or suspended.
unsafe fn step(generator: *mut GeneratorObject) -> Step {
    // A step is a call the call graph does not see, so each one checks the stack, as a call
    // of a recursive function does (FUN-02).
    let exhausted_stack = crate::stack::typy_rt_check_stack();
    if !exhausted_stack.is_null() {
        return Step::Raised(exhausted_stack);
    }
    // SAFETY: the caller's guarantee.
    let object = unsafe { &mut *generator };
    // SAFETY: the current item is owned by the generator, and the descriptor says its kind.
    unsafe { release_current(object) };
    object.state = RUNNING;
    object.caller = crate::exception::handled_exception();
    // SAFETY: the descriptor's code is the resume code of this generator's function.
    let raised = unsafe { ((*object.descriptor).step)(generator) };
    // A step that stops inside an `except` clause of the body leaves the exception it handles
    // with the generator, which handles it again when it resumes (FUN-07).
    // SAFETY: the caller's exception outlives its call of the step.
    unsafe { crate::exception::typy_rt_exception_leave(object.caller) };
    if !raised.is_null() {
        object.state = FINISHED;
        return Step::Raised(escaped(raised));
    }
    if object.state == SUSPENDED {
        return Step::Yielded;
    }
    object.state = FINISHED;
    Step::Returned
}

/// Returns `ValueError` for a generator stepped from inside its own step.
fn running() -> *mut ExceptionObject {
    raise(ExceptionKind::ValueError, "generator already executing")
}

/// Runs the next step of `generator` and returns true when it yielded an item, which
/// `item` then gives; false when its body ended, and then `StopIteration` when `stop` is
/// the `int` word of 1. A generator whose step is running raises `ValueError`, and a
/// `StopIteration` its body raised becomes `RuntimeError`, as CPython has it (PEP 479).
///
/// # Safety
///
/// `generator` points to a live generator object.
pub(crate) unsafe fn advance(generator: *mut GeneratorObject, stop: i64) -> Raising<bool> {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &mut *generator };
    match object.state {
        RUNNING => return Raising::error(false, running()),
        FINISHED => return exhausted(stop),
        _ => {}
    }
    object.received = 0;
    // SAFETY: the generator is new or suspended.
    match unsafe { step(generator) } {
        Step::Yielded => Raising::ok(true),
        Step::Returned => exhausted(stop),
        Step::Raised(exception) => Raising::error(false, exception),
    }
}

/// `generator.send(value)`: runs the next step with `sent`, the bits of the value, borrowed
/// for the step, as the value of the `yield` it resumes; returns true, its item the current
/// one, or raises `StopIteration` when the body ends. A value other than `None`, which
/// `none` says, sent to a generator not started raises `TypeError`, as CPython's does.
///
/// # Safety
///
/// `generator` points to a live generator, and `sent` is a value of the type it is sent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_send(
    generator: *mut GeneratorObject,
    sent: u64,
    none: bool,
) -> Raising<bool> {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &mut *generator };
    match object.state {
        RUNNING => return Raising::error(false, running()),
        FINISHED => return stopped(),
        NEW if !none => {
            return Raising::error(
                false,
                raise(
                    ExceptionKind::TypeError,
                    "can't send non-None value to a just-started generator",
                ),
            );
        }
        NEW => object.received = 0,
        _ => {
            object.received = 1;
            object.sent = sent;
        }
    }
    // SAFETY: the generator is new or suspended.
    match unsafe { step(generator) } {
        Step::Yielded => Raising::ok(true),
        Step::Returned => stopped(),
        Step::Raised(exception) => Raising::error(false, exception),
    }
}

/// `generator.throw(exception)`: raises `exception`, owned, at the `yield` the generator is
/// suspended at, and returns as `send` does. A generator not started, or finished, is
/// finished, and the exception leaves this call, as CPython's `throw` raises it.
///
/// # Safety
///
/// `generator` points to a live generator, and `exception` to a live exception the caller
/// gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_throw(
    generator: *mut GeneratorObject,
    exception: *mut ExceptionObject,
) -> Raising<bool> {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &mut *generator };
    match object.state {
        RUNNING => {
            // SAFETY: the caller gave up its reference.
            unsafe { crate::refcount::typy_rt_decrement(exception.cast()) };
            return Raising::error(false, running());
        }
        NEW | FINISHED => {
            object.state = FINISHED;
            return Raising::error(false, exception);
        }
        _ => {}
    }
    object.received = 0;
    object.thrown = exception;
    // SAFETY: the generator is suspended.
    match unsafe { step(generator) } {
        Step::Yielded => Raising::ok(true),
        Step::Returned => stopped(),
        Step::Raised(raised) => Raising::error(false, raised),
    }
}

/// `generator.close()`: raises `GeneratorExit` at the `yield` a suspended generator is at.
/// Returns null when its body ends, by returning or by raising `GeneratorExit`; a body that
/// yields again gives `RuntimeError: generator ignored GeneratorExit`, and any other
/// exception leaves the call. A generator not started, or finished, is finished (FUN-11).
///
/// # Safety
///
/// `generator` points to a live generator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_close(
    generator: *mut GeneratorObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &mut *generator };
    match object.state {
        RUNNING => return running(),
        NEW | FINISHED => {
            object.state = FINISHED;
            return core::ptr::null_mut();
        }
        _ => {}
    }
    object.received = 0;
    object.thrown = raise_bare(ExceptionKind::GeneratorExit);
    // SAFETY: the generator is suspended.
    match unsafe { step(generator) } {
        Step::Yielded => raise(
            ExceptionKind::RuntimeError,
            "generator ignored GeneratorExit",
        ),
        Step::Returned => core::ptr::null_mut(),
        Step::Raised(raised) => {
            // SAFETY: the step gave up the exception it raised.
            let class = unsafe { (*raised).class() };
            if crate::exception::classes::derives_from(class, ExceptionKind::GeneratorExit.class())
            {
                // SAFETY: the exception is owned here.
                unsafe { crate::refcount::typy_rt_decrement(raised.cast()) };
                return core::ptr::null_mut();
            }
            raised
        }
    }
}

/// Returns the item the last step of `generator` yielded, borrowed until the next step.
///
/// # Safety
///
/// `generator` points to a live generator whose last step yielded.
pub(crate) unsafe fn current(generator: *const GeneratorObject) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*generator).current }
}

/// Makes `item`, which a `yield` gives with its reference, the current item of the running
/// `generator`. An item held inline arrives as the address of its bytes, which the
/// generator copies into room of its own, as a list copies one it stores.
///
/// # Safety
///
/// `generator` points to the live generator whose step is running, and `item` is an item of
/// the kind its descriptor gives.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_hold(generator: *mut GeneratorObject, item: u64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let object = &mut *generator;
        let descriptor = &*object.descriptor;
        let kind =
            crate::list::ItemKind::from_code(u32::try_from(descriptor.item_kind).unwrap_or(0));
        if !matches!(
            kind,
            crate::list::ItemKind::Tuple
                | crate::list::ItemKind::OptionalTuple
                | crate::list::ItemKind::Union
        ) {
            object.current = item;
            return;
        }
        let size = usize::try_from((*descriptor.item_tuple).size).unwrap_or(0);
        if object.held.is_null() {
            object.held = crate::allocator::allocate(size);
        }
        let from = core::ptr::with_exposed_provenance::<u8>(usize::try_from(item).unwrap_or(0));
        core::ptr::copy_nonoverlapping(from, object.held, size);
        object.current = object.held.expose_provenance() as u64;
    }
}

/// Returns the answer of a step that found no item: false, or `StopIteration` for `next()`.
fn exhausted(stop: i64) -> Raising<bool> {
    if crate::word::inline_value(stop) == 1 {
        Raising::error(false, raise_bare(ExceptionKind::StopIteration))
    } else {
        Raising::ok(false)
    }
}

/// Returns `StopIteration` for a `send` or a `throw` that found no item.
fn stopped() -> Raising<bool> {
    Raising::error(false, raise_bare(ExceptionKind::StopIteration))
}

/// Returns the exception a step raised, a `StopIteration` replaced by CPython's
/// `RuntimeError: generator raised StopIteration`, raised from it.
fn escaped(raised: *mut ExceptionObject) -> *mut ExceptionObject {
    // SAFETY: the step returned a live exception it owns.
    let class = unsafe { (*raised).class() };
    if !crate::exception::classes::derives_from(class, ExceptionKind::StopIteration.class()) {
        return raised;
    }
    let replaced = raise(
        ExceptionKind::RuntimeError,
        "generator raised StopIteration",
    );
    // SAFETY: both exceptions are live; the new one takes over the step's reference.
    unsafe {
        let context = (*replaced).context;
        if !context.is_null() {
            crate::refcount::typy_rt_decrement(context.cast());
        }
        (*replaced).cause = raised;
        (*replaced).context = core::ptr::null_mut();
        (*replaced).suppress_context = 1;
    }
    replaced
}

/// Releases the current item of `generator` and leaves none.
///
/// # Safety
///
/// The generator is live, and its current item is zero or owned by it.
unsafe fn release_current(generator: &mut GeneratorObject) {
    let item = core::mem::replace(&mut generator.current, 0);
    if item == 0 {
        return;
    }
    // SAFETY: the descriptor says how the generator holds its items.
    unsafe {
        let descriptor = &*generator.descriptor;
        let kind = u32::try_from(descriptor.item_kind).unwrap_or(0);
        ItemType::of(kind, descriptor.item_tuple).release(item);
    }
}

/// Frees a generator whose last reference is gone, after releasing its current item, the
/// arguments its frame still holds, and, unless its body ended and released them, the
/// variables of its body. A suspended one waits in the queue of dropped generators instead,
/// to be closed first (FUN-11).
///
/// # Safety
///
/// `generator` is a generator object whose last reference is gone.
pub(crate) unsafe fn destroy(generator: *mut GeneratorObject) {
    // SAFETY: the caller's guarantee; the descriptor lays out the frame after the prefix.
    unsafe {
        if (*generator).state == SUSPENDED {
            dropped::push(generator);
            return;
        }
        let object = &mut *generator;
        release_current(object);
        let descriptor = &*object.descriptor;
        let frame = (generator.expose_provenance() + PREFIX) as u64;
        let arguments = usize::try_from(descriptor.arguments).unwrap_or(0);
        let held = if object.state == FINISHED {
            arguments
        } else {
            usize::MAX
        };
        crate::tuple::release_first(descriptor.frame, frame, held);
        if !object.thrown.is_null() {
            crate::refcount::typy_rt_decrement(object.thrown.cast());
        }
        if !object.held.is_null() {
            crate::allocator::free(object.held);
        }
    }
    crate::allocator::free(generator.cast());
}

/// Closes each generator freed while suspended, in the order they were freed, then frees it
/// (FUN-11, ADR-0064). Compiled code calls this when the queue is not empty, after the call
/// that dropped them. The queue is taken whole first, so one generator closes completely,
/// with any its own closing drops, before the next, as CPython frees them one at a time. An
/// exception closing one raises, or a body that yields again, is written to standard error
/// as CPython writes it, and the program goes on.
///
/// # Safety
///
/// Every generator in the queue is a suspended one whose last reference is gone.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_generator_close_dropped() {
    let mut next = dropped::take();
    while !next.is_null() {
        let generator = next;
        // SAFETY: nothing else refers to the generator, so the reference closing holds is
        // its only one; its link is read before it is freed.
        unsafe {
            next = (*generator).dropped;
            (*generator).header.reference_count = 1;
            let raised = typy_rt_generator_close(generator);
            if !raised.is_null() {
                let repr = generator_repr(generator.cast());
                if repr.exception.is_null() {
                    crate::exception::report_ignored(repr.value, raised);
                    crate::refcount::typy_rt_decrement(repr.value.cast());
                } else {
                    crate::refcount::typy_rt_decrement(repr.exception.cast());
                }
                crate::refcount::typy_rt_decrement(raised.cast());
            }
            if (*generator).state == SUSPENDED {
                (*generator).state = ABANDONED;
            }
            crate::refcount::typy_rt_decrement(generator.cast());
        }
    }
}

/// The type `generator`.
#[unsafe(export_name = "typy_rt_type_generator")]
pub(crate) static GENERATOR: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(generator_repr),
    ..TypeObject::new("generator", &raw const OBJECT)
};

/// Returns `repr` of a generator: `<generator object QUALNAME at 0x...>`, with the object's
/// address, as CPython writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a generator object comes from malloc, aligned for every type"
)]
unsafe extern "C" fn generator_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut digits = [0_u8; 16];
    let address = crate::instance::address_digits(object, &mut digits);
    let mut text = StrBuilder::with_capacity(40);
    crate::function::push_ascii(&mut text, "<generator object ");
    // SAFETY: the type of the object says it is a generator, whose descriptor and names are
    // immortal statics.
    unsafe {
        text.push_str((*(*object.cast::<GeneratorObject>()).descriptor).qualified_name);
    }
    crate::function::push_ascii(&mut text, " at 0x");
    crate::function::push_ascii(&mut text, address);
    crate::function::push_ascii(&mut text, ">");
    Raising::ok(text.finish())
}
