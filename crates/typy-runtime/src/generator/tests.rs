use super::{
    FINISHED, GeneratorDescriptor, GeneratorObject, NEW, PREFIX, SUSPENDED, advance, current,
    generator_repr, typy_rt_generator_new,
};
use crate::exception::{ExceptionKind, ExceptionObject};
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
