//! `ctypes` objects: a scalar such as `c_int(5)`, a pointer and an array, each a counted
//! object over a C buffer it owns or shares with the object that owns it (FFI-03,
//! ADR-0048).

mod kept;
#[cfg(test)]
mod tests;

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::header::{CDATA_TYPE_ID, ObjectHeader};
use crate::message::MessageBuffer;
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};
use crate::word::{inline, inline_value, is_inline};

/// What a `ctypes` class is, as `CDataClass::kind` holds it.
pub(crate) const SCALAR: u32 = 0;
/// A pointer class, whose buffer holds an address.
pub(crate) const POINTER: u32 = 1;
/// An array class, whose buffer holds its items.
pub(crate) const ARRAY: u32 = 2;

/// The C types of a scalar class, as `CDataClass::ctype` holds them; the compiler's
/// `CType` numbers them alike.
mod ctype {
    pub(crate) const INT8: u32 = 0;
    pub(crate) const UINT8: u32 = 1;
    pub(crate) const INT16: u32 = 2;
    pub(crate) const UINT16: u32 = 3;
    pub(crate) const INT32: u32 = 4;
    pub(crate) const UINT32: u32 = 5;
    pub(crate) const INT64: u32 = 6;
    pub(crate) const UINT64: u32 = 7;
    pub(crate) const FLOAT: u32 = 8;
    pub(crate) const DOUBLE: u32 = 9;
    pub(crate) const BOOL: u32 = 10;
    pub(crate) const CHAR_POINTER: u32 = 11;
    pub(crate) const VOID_POINTER: u32 = 12;
}

/// A `ctypes` class, which the compiler emits once for each one a program uses.
#[repr(C)]
#[derive(Debug)]
pub struct CDataClass {
    /// The UTF-8 bytes of CPython's name for it, such as `c_long` or `LP_c_int`.
    pub name: *const u8,
    /// The number of bytes of the name.
    pub name_length: u64,
    /// `SCALAR`, `POINTER` or `ARRAY`.
    pub kind: u32,
    /// The C type of a scalar class.
    pub ctype: u32,
    /// The bytes of an object's buffer.
    pub size: u64,
    /// The class of a pointer's target, or of an array's items.
    pub item: *const CDataClass,
    /// The number of an array's items.
    pub length: u64,
}

// SAFETY: a class is a constant the compiler emits, which nothing writes.
unsafe impl Sync for CDataClass {}

impl CDataClass {
    fn name(&self) -> &str {
        // SAFETY: the compiler emits the name as UTF-8 of `name_length` bytes.
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                self.name,
                usize::try_from(self.name_length).unwrap_or(0),
            ))
        }
    }

    fn item(&self) -> &Self {
        // SAFETY: a pointer or array class has an item class the compiler emitted.
        unsafe { &*self.item }
    }
}

/// A `ctypes` object.
#[repr(C)]
#[derive(Debug)]
pub struct CDataObject {
    /// The object header, with the `ctypes` type ID.
    pub header: ObjectHeader,
    /// The object's class.
    pub class: *const CDataClass,
    /// Where its value lives: its own buffer, which follows this fixed part, or memory
    /// another object or C owns.
    pub address: *mut u8,
    /// The object that owns the memory, kept alive while this object is: the object whose
    /// buffer it is, or the pointer it was read through when the memory is C's; null for
    /// the object's own buffer or memory C gave.
    pub owner: *mut ObjectHeader,
    /// The objects the values in its buffer refer to, kept alive with it: a pointer's
    /// target, and the `bytes` of each `c_char_p`; null until it keeps one.
    kept: *mut kept::Kept,
}

const PREFIX: usize = core::mem::size_of::<CDataObject>();

/// The type object of every `ctypes` object, whose slots read the object's own class.
pub(crate) static CDATA: TypeObject = TypeObject {
    truth: Some(cdata_truth),
    repr: Some(cdata_repr),
    ..TypeObject::new("CData", &raw const OBJECT)
};

fn class_of(object: *const CDataObject) -> &'static CDataClass {
    // SAFETY: a live object's class is a static the compiler emitted.
    unsafe { &*(*object).class }
}

/// Returns a new object of `class` with its own buffer, every byte zero, as `T()` makes.
///
/// # Safety
///
/// `class` is a class the compiler emitted.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_new(class: *const CDataClass) -> *mut CDataObject {
    // SAFETY: the caller's guarantee.
    let size = usize::try_from(unsafe { (*class).size }).unwrap_or(0);
    let length = PREFIX + size.next_multiple_of(8);
    let object = allocate(length);
    // SAFETY: the allocation holds `length` bytes, aligned for every type.
    unsafe {
        core::ptr::write_bytes(object.cast::<u8>(), 0, length);
        object.write(CDataObject {
            header: ObjectHeader::new(CDATA_TYPE_ID),
            class,
            address: object.cast::<u8>().add(PREFIX),
            owner: core::ptr::null_mut(),
            kept: core::ptr::null_mut(),
        });
    }
    object
}

/// Returns `length` bytes of the heap for an object.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "the allocator returns memory aligned for every type"
)]
fn allocate(length: usize) -> *mut CDataObject {
    crate::allocator::allocate(length).cast::<CDataObject>()
}

/// Returns the object behind a header whose type is `CDATA`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a ctypes object comes from the allocator, aligned for its fields"
)]
const fn as_cdata(object: *mut ObjectHeader) -> *mut CDataObject {
    object.cast::<CDataObject>()
}

/// Returns a new object of `class` over memory at `address`, which keeps `owner` alive: the
/// object whose memory it is, or the pointer it was read through.
///
/// # Safety
///
/// `class` is a class the compiler emitted, and `owner` a live `ctypes` object.
unsafe fn view(
    class: *const CDataClass,
    address: *mut u8,
    owner: *mut ObjectHeader,
) -> *mut CDataObject {
    let object = allocate(PREFIX);
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_increment(owner) };
    // SAFETY: the allocation holds the fixed part, aligned for it.
    unsafe {
        object.write(CDataObject {
            header: ObjectHeader::new(CDATA_TYPE_ID),
            class,
            address,
            owner,
            kept: core::ptr::null_mut(),
        });
    }
    object
}

/// Frees an object whose last reference is gone, after releasing what it keeps alive.
///
/// # Safety
///
/// `object` is a `ctypes` object no reference is left to.
pub(crate) unsafe fn destroy(object: *mut CDataObject) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let owner = (*object).owner;
        if !owner.is_null() {
            crate::refcount::typy_rt_decrement(owner);
        }
        kept::release_all((*object).kept);
    }
    crate::allocator::free(object.cast());
}

/// Returns the owner whose memory an object's buffer is: the object itself when it has no
/// owner, since it then owns its buffer.
fn memory_owner(object: *mut CDataObject) -> *mut ObjectHeader {
    // SAFETY: the object is live.
    let owner = unsafe { (*object).owner };
    if owner.is_null() {
        object.cast()
    } else {
        owner
    }
}

/// Returns the bytes of a scalar of C type `ctype` at `address`, read as a signed or an
/// unsigned 64-bit value.
///
/// # Safety
///
/// `address` holds a value of `ctype`.
unsafe fn read_integer(address: *const u8, ctype: u32) -> i128 {
    // SAFETY: the caller's guarantee; a C buffer need not be aligned for its type.
    unsafe {
        match ctype {
            ctype::INT8 => i128::from(address.cast::<i8>().read_unaligned()),
            ctype::UINT8 => i128::from(address.read_unaligned()),
            ctype::INT16 => i128::from(address.cast::<i16>().read_unaligned()),
            ctype::UINT16 => i128::from(address.cast::<u16>().read_unaligned()),
            ctype::INT32 => i128::from(address.cast::<i32>().read_unaligned()),
            ctype::UINT32 => i128::from(address.cast::<u32>().read_unaligned()),
            ctype::INT64 => i128::from(address.cast::<i64>().read_unaligned()),
            ctype::UINT64 => i128::from(address.cast::<u64>().read_unaligned()),
            // A scalar of no integer type has no integer value.
            _ => 0,
        }
    }
}

/// Writes the low bits of `bits` at `address`, as many as `ctype` has.
///
/// # Safety
///
/// `address` holds room for a value of `ctype`.
unsafe fn write_integer(address: *mut u8, ctype: u32, bits: i64) {
    let bytes = bits.to_le_bytes();
    let width = match ctype {
        ctype::INT8 | ctype::UINT8 => 1,
        ctype::INT16 | ctype::UINT16 => 2,
        ctype::INT32 | ctype::UINT32 => 4,
        _ => 8,
    };
    // SAFETY: the caller's guarantee.
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), address, width) };
}

/// Returns the value of an integer scalar as an `int` word the caller owns.
///
/// # Safety
///
/// `object` is a live scalar of an integer type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_get_int(object: *mut CDataObject) -> i64 {
    // SAFETY: the caller's guarantee.
    let value = unsafe { read_integer((*object).address, class_of(object).ctype) };
    crate::int::value::Integer::from_i128(value).into_word()
}

/// Assigns an integer scalar the low bits of an `int` word, as `ctypes` wraps it.
///
/// # Safety
///
/// `object` is a live scalar of an integer type, and `word` an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_set_int(object: *mut CDataObject, word: i64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let bits = crate::foreign::typy_rt_int_low_bits(word);
        write_integer((*object).address, class_of(object).ctype, bits);
    }
}

/// Returns the value of a `c_float` or `c_double`.
///
/// # Safety
///
/// `object` is a live scalar of a floating type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_get_float(object: *mut CDataObject) -> f64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let address = (*object).address;
        if class_of(object).ctype == ctype::FLOAT {
            f64::from(address.cast::<f32>().read_unaligned())
        } else {
            address.cast::<f64>().read_unaligned()
        }
    }
}

/// Assigns a `c_float`, rounded to 32 bits, or a `c_double`.
///
/// # Safety
///
/// `object` is a live scalar of a floating type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_set_float(object: *mut CDataObject, value: f64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let address = (*object).address;
        if class_of(object).ctype == ctype::FLOAT {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a `c_float` holds the value rounded to 32 bits, as C does"
            )]
            address.cast::<f32>().write_unaligned(value as f32);
        } else {
            address.cast::<f64>().write_unaligned(value);
        }
    }
}

/// Returns the value of a `c_bool`: whether its byte is not zero.
///
/// # Safety
///
/// `object` is a live `c_bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_get_bool(object: *mut CDataObject) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { (*object).address.read() != 0 }
}

/// Assigns a `c_bool`.
///
/// # Safety
///
/// `object` is a live `c_bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_set_bool(object: *mut CDataObject, value: bool) {
    // SAFETY: the caller's guarantee.
    unsafe { (*object).address.write(u8::from(value)) };
}

/// Returns the address a `c_char_p`, `c_void_p` or pointer holds.
///
/// # Safety
///
/// `object` is a live object whose buffer holds an address.
unsafe fn held_address(object: *mut CDataObject) -> *mut u8 {
    // SAFETY: the caller's guarantee.
    unsafe { (*object).address.cast::<*mut u8>().read_unaligned() }
}

/// Returns the value of a `c_char_p`: a copy of its C string as `bytes` the caller owns, or
/// null for `None`.
///
/// # Safety
///
/// `object` is a live `c_char_p` whose address is null or a C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_get_bytes(object: *mut CDataObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    unsafe { crate::foreign::typy_rt_bytes_from_c_string(held_address(object)) }
}

/// Assigns a `c_char_p` the buffer of `bytes`, which it keeps alive, or null for null.
///
/// # Safety
///
/// `object` is a live `c_char_p`, and `bytes` null or a live `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_set_bytes(object: *mut CDataObject, bytes: *mut StrObject) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let data = crate::string::typy_rt_bytes_data(bytes);
        let address = (*object).address;
        address.cast::<*const u8>().write_unaligned(data);
        kept::keep(&mut (*keeper(object)).kept, address, bytes.cast());
    }
}

/// Returns the value of a `c_void_p`: its address as an `int` word the caller owns, or 0,
/// `None`, for null.
///
/// # Safety
///
/// `object` is a live `c_void_p`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_get_address(object: *mut CDataObject) -> i64 {
    // SAFETY: the caller's guarantee.
    crate::foreign::typy_rt_int_from_address(unsafe { held_address(object) })
}

/// Assigns a `c_void_p` the low 64 bits of an `int` word, or null for 0, `None`.
///
/// # Safety
///
/// `object` is a live `c_void_p`, and `word` an `int` word or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_set_address(object: *mut CDataObject, word: i64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let bits = crate::foreign::typy_rt_int_low_bits(word);
        (*object).address.cast::<i64>().write_unaligned(bits);
    }
}

/// Returns a new pointer of `class` to `target`, which it keeps alive, as `pointer(x)`
/// makes it.
///
/// # Safety
///
/// `class` is a pointer class the compiler emitted, and `target` a live object of its
/// target class.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_pointer(
    class: *const CDataClass,
    target: *mut CDataObject,
) -> *mut CDataObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let pointer = typy_rt_cdata_new(class);
        let address = (*pointer).address;
        address.cast::<*mut u8>().write_unaligned((*target).address);
        kept::keep(&mut (*pointer).kept, address, target.cast());
        pointer
    }
}

/// Returns a new pointer of `class` that holds `address`, a C function's result, and keeps
/// nothing alive.
///
/// # Safety
///
/// `class` is a pointer class the compiler emitted.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_from_address(
    class: *const CDataClass,
    address: *mut u8,
) -> *mut CDataObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let pointer = typy_rt_cdata_new(class);
        (*pointer)
            .address
            .cast::<*mut u8>()
            .write_unaligned(address);
        pointer
    }
}

/// Returns the object that keeps what is written into `object`'s memory: the object
/// whose memory it is, or itself.
///
/// # Safety
///
/// `object` is a live object, whose owner is null or a live `ctypes` object.
unsafe fn keeper(object: *mut CDataObject) -> *mut CDataObject {
    // SAFETY: the caller's guarantee.
    let owner = unsafe { (*object).owner };
    if owner.is_null() {
        object
    } else {
        owner.cast()
    }
}

/// Returns the object an object read through `pointer` belongs to: the target the pointer
/// keeps, or for memory C gave, the pointer itself.
///
/// # Safety
///
/// `pointer` is a live pointer.
unsafe fn pointed_owner(pointer: *mut CDataObject) -> *mut ObjectHeader {
    // SAFETY: the caller's guarantee.
    let target = unsafe { kept::kept_at((*pointer).kept, (*pointer).address) };
    if target.is_null() {
        pointer.cast()
    } else {
        target
    }
}

fn null_access() -> *mut crate::exception::ExceptionObject {
    raise(ExceptionKind::ValueError, "NULL pointer access")
}

/// Returns `pointer.contents`: a new object of the target class over the memory the
/// pointer holds, which keeps the target alive; `ValueError` for a null pointer.
///
/// # Safety
///
/// `pointer` is a live pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_contents(
    pointer: *mut CDataObject,
) -> Raising<*mut CDataObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let address = held_address(pointer);
        if address.is_null() {
            return Raising::error(core::ptr::null_mut(), null_access());
        }
        Raising::ok(view(
            class_of(pointer).item,
            address,
            pointed_owner(pointer),
        ))
    }
}

/// Returns a new object over item `index` of a pointer or an array: a pointer's item is not
/// checked, and a null pointer is `ValueError`; an array's counts from the end when
/// negative, and one outside it is `IndexError`.
///
/// # Safety
///
/// `object` is a live pointer or array, and `index` an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_item(
    object: *mut CDataObject,
    index: i64,
) -> Raising<*mut CDataObject> {
    let class = class_of(object);
    let item = class.item();
    let size = isize::try_from(item.size).unwrap_or(0);
    let position = if is_inline(index) {
        inline_value(index)
    } else {
        i64::MAX
    };
    // SAFETY: the caller's guarantee.
    unsafe {
        if class.kind == POINTER {
            let address = held_address(object);
            if address.is_null() {
                return Raising::error(core::ptr::null_mut(), null_access());
            }
            let offset = isize::try_from(position)
                .unwrap_or(isize::MAX)
                .wrapping_mul(size);
            let owner = pointed_owner(object);
            return Raising::ok(view(item, address.wrapping_offset(offset), owner));
        }
        let length = i64::try_from(class.length).unwrap_or(i64::MAX);
        let place = if position < 0 {
            position + length
        } else {
            position
        };
        if !(0..length).contains(&place) {
            return Raising::error(
                core::ptr::null_mut(),
                raise(ExceptionKind::IndexError, "invalid index"),
            );
        }
        let offset = isize::try_from(place).unwrap_or(0) * size;
        Raising::ok(view(
            item,
            (*object).address.offset(offset),
            memory_owner(object),
        ))
    }
}

/// Returns the number of an array's items as an `int` word.
///
/// # Safety
///
/// `object` is a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_length(object: *mut CDataObject) -> i64 {
    inline(i64::try_from(class_of(object).length).unwrap_or(0)).unwrap_or(1)
}

/// Returns the bytes of an object's buffer as an `int` word, as `ctypes.sizeof` gives them.
///
/// # Safety
///
/// `object` is a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_size(object: *mut CDataObject) -> i64 {
    inline(i64::try_from(class_of(object).size).unwrap_or(0)).unwrap_or(1)
}

/// Returns the address a C function takes for `object`: null for `None`, the address a
/// pointer holds, and the buffer of a scalar or an array, as `byref` gives it.
///
/// # Safety
///
/// `object` is null or a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cdata_argument(object: *mut CDataObject) -> *mut u8 {
    if object.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: the caller's guarantee.
    unsafe {
        if class_of(object).kind == POINTER {
            held_address(object)
        } else {
            (*object).address
        }
    }
}

/// The truth of an object: a scalar's value's, a pointer's not being null, and an array's
/// having items.
unsafe extern "C" fn cdata_truth(object: *mut ObjectHeader) -> bool {
    let object = as_cdata(object);
    let class = class_of(object);
    // SAFETY: the object is live and its buffer holds a value of its class.
    unsafe {
        match (class.kind, class.ctype) {
            (ARRAY, _) => class.length > 0,
            (POINTER, _) | (_, ctype::CHAR_POINTER | ctype::VOID_POINTER) => {
                !held_address(object).is_null()
            }
            (_, ctype::FLOAT | ctype::DOUBLE) => typy_rt_cdata_get_float(object) != 0.0,
            (_, ctype::BOOL) => typy_rt_cdata_get_bool(object),
            (_, ctype) => read_integer((*object).address, ctype) != 0,
        }
    }
}

/// Returns `repr(object)` as CPython 3.15 writes it: `c_long(7)` for a scalar, and the
/// class and the address for a pointer or an array.
unsafe extern "C" fn cdata_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let object = as_cdata(object);
    let class = class_of(object);
    let mut text = MessageBuffer::new();
    if class.kind != SCALAR {
        let module = if class.kind == POINTER {
            "ctypes"
        } else {
            "__main__"
        };
        text.push_str("<");
        text.push_str(module);
        text.push_str(".");
        text.push_str(class.name());
        text.push_str(" object at 0x");
        push_address(&mut text, object.addr());
        text.push_str(">");
        return Raising::ok(text.text());
    }
    text.push_str(class.name());
    text.push_str("(");
    // SAFETY: the object is live and its buffer holds a value of its class.
    unsafe {
        match class.ctype {
            ctype::FLOAT | ctype::DOUBLE => {
                let shown =
                    crate::float::repr::typy_rt_float_to_str(typy_rt_cdata_get_float(object));
                text.push_text(shown);
                crate::refcount::typy_rt_decrement(shown.cast());
            }
            ctype::BOOL => text.push_str(if typy_rt_cdata_get_bool(object) {
                "True"
            } else {
                "False"
            }),
            ctype::CHAR_POINTER | ctype::VOID_POINTER => {
                let address = held_address(object);
                if address.is_null() {
                    text.push_str("None");
                } else {
                    push_decimal(&mut text, i128::from(address.addr() as u64));
                }
            }
            other => push_decimal(&mut text, read_integer((*object).address, other)),
        }
    }
    text.push_str(")");
    Raising::ok(text.text())
}

fn push_decimal(text: &mut MessageBuffer, value: i128) {
    if let Ok(small) = i64::try_from(value) {
        text.push_number(small);
        return;
    }
    // Only a `c_ulong` holds a value past `i64`, so the value here is positive.
    let mut digits = [0_u8; 40];
    let mut rest = value.unsigned_abs();
    let mut at = digits.len();
    while rest > 0 {
        at -= 1;
        digits[at] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        rest /= 10;
    }
    for digit in &digits[at..] {
        text.push(u32::from(*digit));
    }
}

fn push_address(text: &mut MessageBuffer, address: usize) {
    let mut started = false;
    for shift in (0..16).rev() {
        let digit = (address >> (shift * 4)) & 0xf;
        if digit != 0 || started || shift == 0 {
            started = true;
            text.push(u32::from(b"0123456789abcdef"[digit]));
        }
    }
}
