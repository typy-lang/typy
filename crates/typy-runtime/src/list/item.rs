//! The types of list items, and how each is stored, counted, compared and written as text. An
//! item of a fixed kind is eight bytes: an `int` word, the bits of a `float`, a `bool` as 0 or
//! 1, a pointer, or 0 for `None`. A tuple is stored inline, as many words as its descriptor
//! says, and crosses the runtime's interface as the address of its bytes (ADR-0001, ADR-0009).

use crate::Raising;
use crate::header::ObjectHeader;
use crate::string::StrObject;
use crate::tuple::TupleType;
use crate::type_object::type_of;
use crate::word::inline;

/// How a list stores its items. The compiler passes the code of the kind it chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum ItemKind {
    /// An `int` word: inline, or a pointer to a large integer.
    Int = 0,
    /// The bits of a `float`.
    Float = 1,
    /// A `bool`, 0 or 1.
    Bool = 2,
    /// A pointer to a `str` or a `list`.
    Object = 3,
    /// `None`, stored as 0.
    NoneValue = 4,
    /// A tuple, stored inline as its descriptor lays it out.
    Tuple = 5,
    /// An `int | None`: an `int` word, or 0 for `None` (ADR-0011).
    OptionalInt = 6,
    /// A `float | None`: the bits of a `float`, or the reserved signalling NaN for `None`.
    OptionalFloat = 7,
    /// A `bool | None`: 0 or 1, or 2 for `None`.
    OptionalBool = 8,
    /// An object `| None`: a pointer, or null for `None`.
    OptionalObject = 9,
    /// A `tuple | None`: a two-item tuple of a presence flag and the tuple, as its descriptor
    /// lays out the pair.
    OptionalTuple = 10,
}

impl ItemKind {
    /// Returns the kind with this code; an unknown code is a defect in the compiler.
    pub(crate) fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Int,
            1 => Self::Float,
            2 => Self::Bool,
            3 => Self::Object,
            4 => Self::NoneValue,
            5 => Self::Tuple,
            6 => Self::OptionalInt,
            7 => Self::OptionalFloat,
            8 => Self::OptionalBool,
            9 => Self::OptionalObject,
            10 => Self::OptionalTuple,
            _ => crate::libc::abort_with(b"typy runtime: a list has an unknown item kind\n"),
        }
    }
}

/// The type of a list's items: a kind, and for a tuple its descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemType {
    kind: ItemKind,
    tuple: *const TupleType,
}

impl From<ItemKind> for ItemType {
    fn from(kind: ItemKind) -> Self {
        Self {
            kind,
            tuple: core::ptr::null(),
        }
    }
}

impl ItemType {
    /// Returns the type whose kind has the code `kind`, with the descriptor `tuple` for tuples.
    pub(crate) fn of(kind: u32, tuple: *const TupleType) -> Self {
        match ItemKind::from_code(kind) {
            ItemKind::Tuple => Self::tuple(tuple),
            ItemKind::OptionalTuple => Self {
                kind: ItemKind::OptionalTuple,
                tuple,
            },
            kind => Self::from(kind),
        }
    }

    /// Returns the type of the values an optional item holds when it is not `None`, or `None`
    /// for a type that is not optional.
    pub(crate) fn base(self) -> Option<Self> {
        Some(match self.kind {
            ItemKind::OptionalInt => Self::from(ItemKind::Int),
            ItemKind::OptionalFloat => Self::from(ItemKind::Float),
            ItemKind::OptionalBool => Self::from(ItemKind::Bool),
            ItemKind::OptionalObject => Self::from(ItemKind::Object),
            // SAFETY: an optional tuple's descriptor is the pair of its flag and its tuple.
            ItemKind::OptionalTuple => unsafe {
                Self::tuple(crate::tuple::field_tuple(self.tuple, 1))
            },
            _ => return None,
        })
    }

    /// Returns whether an optional item is `None`.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this optional type.
    unsafe fn is_none(self, item: u64) -> bool {
        match self.kind {
            ItemKind::OptionalFloat => item == NONE_FLOAT,
            ItemKind::OptionalBool => item == 2,
            // SAFETY: the caller passes the address of a live pair; its flag is item 0.
            ItemKind::OptionalTuple => unsafe { crate::tuple::field(self.tuple, item, 0) == 0 },
            _ => item == 0,
        }
    }

    /// Returns the value an optional item holds that is not `None`: the item itself, or for a
    /// tuple the address of the tuple inside the pair.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this optional type.
    unsafe fn unwrapped(self, item: u64) -> u64 {
        match self.kind {
            // SAFETY: the caller passes the address of a live pair; its tuple is item 1.
            ItemKind::OptionalTuple => unsafe { crate::tuple::field(self.tuple, item, 1) },
            _ => item,
        }
    }

    /// Returns what an item holds: itself for a type that is not optional, `None`, or the value
    /// of an optional item with the value's type.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    unsafe fn held(self, item: u64) -> Held {
        let Some(base) = self.base() else {
            return Held::Plain;
        };
        // SAFETY: the caller's guarantee.
        unsafe {
            if self.is_none(item) {
                Held::None
            } else {
                Held::Value(base, self.unwrapped(item))
            }
        }
    }

    /// Returns the type of tuple items laid out by `tuple`.
    pub(crate) const fn tuple(tuple: *const TupleType) -> Self {
        Self {
            kind: ItemKind::Tuple,
            tuple,
        }
    }

    /// Returns the kind.
    pub(crate) const fn kind(self) -> ItemKind {
        self.kind
    }

    /// Returns the descriptor of tuple items, or null.
    pub(crate) const fn descriptor(self) -> *const TupleType {
        self.tuple
    }

    /// Returns the number of eight-byte words an item takes.
    pub(crate) fn words(self) -> usize {
        match self.kind {
            // SAFETY: a tuple item type always has a live descriptor, emitted by the compiler.
            ItemKind::Tuple | ItemKind::OptionalTuple => unsafe { (*self.tuple).words() },
            _ => 1,
        }
    }

    /// Returns the item stored at `slot`: its eight bytes, or for a tuple the slot's address.
    ///
    /// # Safety
    ///
    /// `slot` points to an item of this type.
    pub(crate) unsafe fn load(self, slot: *const u64) -> u64 {
        match self.kind {
            ItemKind::Tuple | ItemKind::OptionalTuple => slot.expose_provenance() as u64,
            // SAFETY: the caller passes a slot that holds an item.
            _ => unsafe { *slot },
        }
    }

    /// Writes `item` into `slot`, without counting a reference: its eight bytes, or for a
    /// tuple the words at the address `item` holds.
    ///
    /// # Safety
    ///
    /// `slot` has room for an item of this type, and `item` is one, which does not overlap it.
    pub(crate) unsafe fn store(self, slot: *mut u64, item: u64) {
        // SAFETY: the caller passes a slot with room for the item and a live item.
        unsafe {
            match self.kind {
                ItemKind::Tuple | ItemKind::OptionalTuple => {
                    core::ptr::copy_nonoverlapping(address(item), slot, self.words());
                }
                _ => *slot = item,
            }
        }
    }

    /// Adds a reference to every object an item holds.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn retain(self, item: u64) {
        // SAFETY: the caller passes a live item; its bits are an `int` word, a pointer or an
        // address.
        unsafe {
            match self.kind {
                ItemKind::Int | ItemKind::OptionalInt => {
                    crate::refcount::typy_rt_int_increment(item.cast_signed());
                }
                ItemKind::Object | ItemKind::OptionalObject => {
                    crate::refcount::typy_rt_increment(pointer(item));
                }
                ItemKind::Tuple | ItemKind::OptionalTuple => crate::tuple::retain(self.tuple, item),
                ItemKind::Float
                | ItemKind::Bool
                | ItemKind::NoneValue
                | ItemKind::OptionalFloat
                | ItemKind::OptionalBool => {}
            }
        }
    }

    /// Drops a reference to every object an item holds.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type that the caller holds references for.
    pub(crate) unsafe fn release(self, item: u64) {
        // SAFETY: the caller passes a live item it holds references for.
        unsafe {
            match self.kind {
                ItemKind::Int | ItemKind::OptionalInt => {
                    crate::refcount::typy_rt_int_decrement(item.cast_signed());
                }
                ItemKind::Object | ItemKind::OptionalObject => {
                    crate::refcount::typy_rt_decrement(pointer(item));
                }
                ItemKind::Tuple | ItemKind::OptionalTuple => {
                    crate::tuple::release(self.tuple, item);
                }
                ItemKind::Float
                | ItemKind::Bool
                | ItemKind::NoneValue
                | ItemKind::OptionalFloat
                | ItemKind::OptionalBool => {}
            }
        }
    }

    /// Returns `left == right` for two items of this type, as CPython compares them.
    #[allow(
        clippy::float_cmp,
        reason = "Python's `==` on floats is IEEE equality, under which `nan` is not equal to itself"
    )]
    ///
    /// # Safety
    ///
    /// Both are live items of this type.
    pub(crate) unsafe fn equal(self, left: u64, right: u64) -> bool {
        // SAFETY: the caller passes live items of this type.
        match unsafe { (self.held(left), self.held(right)) } {
            (Held::Plain, _) | (_, Held::Plain) => {}
            (Held::None, Held::None) => return true,
            (Held::Value(base, left), Held::Value(_, right)) => {
                // SAFETY: both are live values of the optional type's base.
                return unsafe { base.equal(left, right) };
            }
            _ => return false,
        }
        let equal_code = inline(0).unwrap_or(1);
        match self.kind {
            ItemKind::Bool => left == right,
            ItemKind::NoneValue => true,
            ItemKind::Float => f64::from_bits(left) == f64::from_bits(right),
            // SAFETY: the caller passes live `int` words.
            ItemKind::Int => unsafe {
                crate::int::typy_rt_int_compare(left.cast_signed(), right.cast_signed(), equal_code)
            },
            // SAFETY: the caller passes pointers to live objects of one type.
            ItemKind::Object => unsafe { objects_equal(pointer(left), pointer(right)) },
            // SAFETY: the caller passes the addresses of two live tuples of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::equal(self.tuple, left, right) },
            _ => unreachable!("an optional item is compared by its base"),
        }
    }

    /// Returns whether `left` and `right` stand in the comparison whose code is the `int` word
    /// `comparison`, as CPython orders two items of this type: numbers by value, and strings,
    /// lists and tuples item by item. `None` is not ordered; the checker refuses it.
    ///
    /// # Safety
    ///
    /// Both are live items of this type.
    #[allow(
        clippy::float_cmp,
        reason = "Python's comparisons on floats are IEEE comparisons, under which `nan` is unordered"
    )]
    pub(crate) unsafe fn order(self, left: u64, right: u64, comparison: i64) -> bool {
        // The checker orders no optional item; only a list whose items are all values reaches
        // here, and two `None` items are equal.
        // SAFETY: the caller passes live items of this type.
        match unsafe { (self.held(left), self.held(right)) } {
            (Held::Plain, _) | (_, Held::Plain) => {}
            (Held::Value(base, left), Held::Value(_, right)) => {
                // SAFETY: both are live values of the optional type's base.
                return unsafe { base.order(left, right, comparison) };
            }
            _ => return crate::compare::holds(core::cmp::Ordering::Equal, comparison),
        }
        match self.kind {
            ItemKind::Bool | ItemKind::NoneValue => {
                crate::compare::holds(left.cmp(&right), comparison)
            }
            ItemKind::Float => {
                let (left, right) = (f64::from_bits(left), f64::from_bits(right));
                match crate::word::inline_value(comparison) {
                    0 => left == right,
                    1 => left != right,
                    2 => left < right,
                    3 => left <= right,
                    4 => left > right,
                    _ => left >= right,
                }
            }
            // SAFETY: the caller passes live `int` words.
            ItemKind::Int => unsafe {
                crate::int::typy_rt_int_compare(left.cast_signed(), right.cast_signed(), comparison)
            },
            // SAFETY: the caller passes pointers to live objects of one type.
            ItemKind::Object => unsafe { objects_order(pointer(left), pointer(right), comparison) },
            // SAFETY: the caller passes the addresses of two live tuples of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::order(self.tuple, left, right, comparison) },
            _ => unreachable!("an optional item is ordered by its base"),
        }
    }

    /// Returns `hash(item)`, the value CPython gives an equal item (ADR-0010). A list is not
    /// hashable, and the checker refuses one where a hash is needed.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn hash(self, item: u64) -> i64 {
        // SAFETY: the caller passes a live item of this type.
        match unsafe { self.held(item) } {
            Held::None => return crate::hash::NONE,
            // SAFETY: a live value of the optional type's base.
            Held::Value(base, value) => return unsafe { base.hash(value) },
            Held::Plain => {}
        }
        match self.kind {
            ItemKind::Bool => i64::from(item != 0),
            ItemKind::NoneValue => crate::hash::NONE,
            ItemKind::Float => crate::hash::float(f64::from_bits(item)),
            // SAFETY: the caller passes a live `int` word.
            ItemKind::Int => unsafe { crate::hash::int(item.cast_signed()) },
            // SAFETY: the caller passes a pointer to a live object.
            ItemKind::Object => unsafe { object_hash(pointer(item)) },
            // SAFETY: the caller passes the address of a live tuple of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::hash(self.tuple, item) },
            _ => unreachable!("an optional item is hashed by its base"),
        }
    }

    /// Returns the truth of an item, as `bool(item)` gives it: a number is true when it is not
    /// zero, a `str`, a list or a tuple when it is not empty, and `None` never.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn is_true(self, item: u64) -> bool {
        // SAFETY: the caller passes a live item of this type.
        match unsafe { self.held(item) } {
            Held::None => return false,
            // SAFETY: a live value of the optional type's base.
            Held::Value(base, value) => return unsafe { base.is_true(value) },
            Held::Plain => {}
        }
        match self.kind {
            ItemKind::Bool => item != 0,
            ItemKind::NoneValue => false,
            // A large integer is never zero, because zero is always inline.
            ItemKind::Int => item.cast_signed() != inline(0).unwrap_or(1),
            ItemKind::Float => f64::from_bits(item) != 0.0,
            // SAFETY: the caller passes a pointer to a live object.
            ItemKind::Object => unsafe { object_is_true(pointer(item)) },
            // SAFETY: a tuple item type always has a live descriptor.
            ItemKind::Tuple => unsafe { (*self.tuple).length != 0 },
            _ => unreachable!("an optional item is tested by its base"),
        }
    }

    /// Returns `repr(item)`; only an `int` of more than 4300 digits raises.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn repr(self, item: u64) -> Raising<*mut StrObject> {
        // SAFETY: the caller passes a live item of this type.
        match unsafe { self.held(item) } {
            Held::None => return Raising::ok(crate::string::from_str("None")),
            // SAFETY: a live value of the optional type's base.
            Held::Value(base, value) => return unsafe { base.repr(value) },
            Held::Plain => {}
        }
        match self.kind {
            // SAFETY: the caller passes a live `int` word.
            ItemKind::Int => unsafe { crate::int::typy_rt_int_to_str(item.cast_signed()) },
            ItemKind::Float => {
                Raising::ok(crate::float::typy_rt_float_to_str(f64::from_bits(item)))
            }
            ItemKind::Bool => Raising::ok(crate::string::from_str(if item == 0 {
                "False"
            } else {
                "True"
            })),
            ItemKind::NoneValue => Raising::ok(crate::string::from_str("None")),
            // SAFETY: the caller passes a pointer to a live object.
            ItemKind::Object => unsafe { object_repr(pointer(item)) },
            // SAFETY: the caller passes the address of a live tuple of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::repr(self.tuple, item) },
            _ => unreachable!("an optional item is written by its base"),
        }
    }
}

impl ItemType {
    /// Returns `str(item)`: the text of a `str`, the `str` of an object whose type writes one,
    /// and otherwise `repr(item)`.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn str(self, item: u64) -> Raising<*mut StrObject> {
        // SAFETY: the caller passes a live item of this type.
        let (kind, value) = match unsafe { self.held(item) } {
            Held::Value(base, value) => (base.kind, value),
            Held::Plain => (self.kind, item),
            Held::None => return Raising::ok(crate::string::from_str("None")),
        };
        if kind != ItemKind::Object {
            // SAFETY: the caller passes a live item of this type.
            return unsafe { self.repr(item) };
        }
        let object = pointer(value);
        // SAFETY: the item points to a live object, and its type's function takes it.
        unsafe {
            if (*object).type_id == crate::header::STR_TYPE_ID {
                crate::refcount::typy_rt_increment(object);
                return Raising::ok(object.cast());
            }
            match type_of(object).str {
                Some(str) => str(object),
                None => object_repr(object),
            }
        }
    }
}

/// What an item holds, as `ItemType::held` reads it.
enum Held {
    /// An item of a type that is not optional.
    Plain,
    /// `None` of an optional item.
    None,
    /// The value an optional item holds, with the value's type.
    Value(ItemType, u64),
}

/// The bits of `None` of a `float | None`: a signalling NaN that no float operation produces,
/// since every operation gives a quiet NaN (ADR-0011).
pub(crate) const NONE_FLOAT: u64 = 0x7ff4_0000_0000_0001;

/// Returns the words an item address points to.
fn address(item: u64) -> *const u64 {
    core::ptr::with_exposed_provenance(usize::try_from(item).unwrap_or(0))
}

/// Returns the object an item points to.
fn pointer(item: u64) -> *mut ObjectHeader {
    core::ptr::with_exposed_provenance_mut(usize::try_from(item).unwrap_or(0))
}

/// Returns whether an object is true: with its type's test, or true when it has none.
///
/// # Safety
///
/// `object` points to a live object.
unsafe fn object_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the caller passes a live object, and its type's function takes it.
    unsafe {
        match type_of(object).truth {
            Some(truth) => truth(object),
            None => true,
        }
    }
}

/// Returns the hash of an object of a hashable type.
///
/// # Safety
///
/// `object` points to a live object.
unsafe fn object_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the caller passes a live object, and its type's function takes it.
    unsafe {
        match type_of(object).hash {
            Some(hash) => hash(object),
            None => crate::libc::abort_with(
                b"typy runtime: a hash was asked of an object without one\n",
            ),
        }
    }
}

/// Returns `left == right` for two objects of one type; without a test of its own, a type's
/// objects are equal only to themselves.
///
/// # Safety
///
/// Both point to live objects of one type.
unsafe fn objects_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the caller passes live objects of one type, and its function takes them.
    unsafe {
        match type_of(left).equal {
            Some(equal) => equal(left, right),
            None => core::ptr::eq(left, right),
        }
    }
}

/// Returns whether two objects of one type stand in the comparison `comparison`; sets order as
/// subsets and supersets.
///
/// # Safety
///
/// Both point to live objects of one ordered type.
unsafe fn objects_order(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
    comparison: i64,
) -> bool {
    // SAFETY: the caller passes live objects of one type, and its function takes them.
    unsafe {
        match type_of(left).order {
            Some(order) => order(left, right, comparison),
            None => {
                crate::libc::abort_with(b"typy runtime: a list holds an object it cannot order\n")
            }
        }
    }
}

/// Returns `repr(object)` with the function of the object's type.
///
/// # Safety
///
/// `object` points to a live object.
unsafe fn object_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live object, and its type's function takes it.
    unsafe {
        match type_of(object).repr {
            Some(repr) => repr(object),
            None => {
                crate::libc::abort_with(b"typy runtime: a list holds an object it cannot write\n")
            }
        }
    }
}
