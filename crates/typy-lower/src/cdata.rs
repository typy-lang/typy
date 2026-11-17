//! Lowers what a program does with `ctypes` classes and objects: each class it uses becomes
//! one entry of the module's table, and each use a call of the runtime's `cdata` functions
//! with the class's address (FFI-03, ADR-0048).

use typy_check::{CData, CDataExpr, CDataPlace, CType, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{CDataShape, Constant, OptionalBase, RuntimeFunction, TyirCDataClass, TyirType};

use crate::foreign_call::c_type;
use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The `ctypes` classes of a module, each once, in the order the lowering first meets them.
#[derive(Default)]
pub(crate) struct CDataClasses {
    classes: Vec<(CData, TyirCDataClass)>,
}

impl CDataClasses {
    /// Returns the index of `class`, adding it, and the class of its item, when new.
    pub(crate) fn intern(&mut self, class: &CData) -> u32 {
        if let Some(index) = self.classes.iter().position(|(each, _)| each == class) {
            return u32::try_from(index).unwrap_or(u32::MAX);
        }
        let shape = match class {
            CData::Scalar(scalar) => CDataShape::Scalar(c_type(*scalar)),
            CData::Pointer(target) => CDataShape::Pointer(self.intern(target)),
            CData::Array(item, length) => CDataShape::Array(self.intern(item), u64::from(*length)),
        };
        self.classes.push((
            class.clone(),
            TyirCDataClass {
                name: class.class_name(),
                shape,
                size: class.size(),
            },
        ));
        u32::try_from(self.classes.len() - 1).unwrap_or(u32::MAX)
    }

    /// Returns the table of the module.
    pub(crate) fn into_table(self) -> Vec<TyirCDataClass> {
        self.classes.into_iter().map(|(_, class)| class).collect()
    }
}

/// Returns the scalar type of what an object of type `ty` holds: its own value, or with
/// `item`, an item's.
fn scalar_of(ty: &Type, item: bool) -> CType {
    let class = class_of(ty);
    let scalar = if item {
        class.item().and_then(CData::scalar)
    } else {
        class.scalar()
    };
    scalar.unwrap_or_else(|| unreachable!("the checker reads only a scalar value or item"))
}

/// Returns the class of a `ctypes` object of type `ty`.
fn class_of(ty: &Type) -> &CData {
    match ty {
        Type::CData(class) => class,
        _ => unreachable!("the checker gives a `ctypes` operation a `ctypes` object"),
    }
}

/// Returns the scalar type of the items of a pointer or an array of type `ty`.
pub(crate) fn item_scalar(ty: &Type) -> CType {
    scalar_of(ty, true)
}

impl FunctionLowerer<'_> {
    /// Returns the address of `class`, as the runtime's functions take it.
    fn class_address(&mut self, class: &CData, span: Span) -> typy_tyir::ValueId {
        let index = self.cdata_classes.intern(class);
        self.constant(Constant::CDataClass(index), span)
    }

    /// Lowers an expression on a `ctypes` class or object, whose result has type `ty`.
    pub(crate) fn cdata_expression(&mut self, cdata: &CDataExpr, ty: &Type, span: Span) -> Lowered {
        match cdata {
            CDataExpr::Class => Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit),
            CDataExpr::New(arguments) => self.new_cdata(ty, arguments, span),
            CDataExpr::Pointer(target) => {
                let class = class_of(ty);
                let target = self.expression(target);
                let class = self.class_address(class, span);
                let pointer = self.call_runtime(
                    RuntimeFunction::CdataPointer,
                    vec![class, target.value],
                    span,
                );
                self.release(target, span);
                pointer
            }
            CDataExpr::Value(object) => {
                let scalar = scalar_of(&object.ty, false);
                let object = self.expression(object);
                let value = self.read_scalar(object.value, scalar, span);
                self.release(object, span);
                value
            }
            CDataExpr::Contents(object) => {
                self.cdata_unary(RuntimeFunction::CdataContents, object, span)
            }
            CDataExpr::Item { object, index } => {
                let scalar = item_scalar(&object.ty);
                let object = self.expression(object);
                let index = self.expression(index);
                let value = self.cdata_item_value(object.value, index.value, scalar, span);
                self.release(index, span);
                self.release(object, span);
                value
            }
            CDataExpr::Length(object) => {
                self.cdata_unary(RuntimeFunction::CdataLength, object, span)
            }
            CDataExpr::Size(object) => self.cdata_unary(RuntimeFunction::CdataSize, object, span),
            CDataExpr::Reference(object) => self.expression(object),
        }
    }

    /// Calls `function` of the runtime with `object` and returns its result.
    fn cdata_unary(
        &mut self,
        function: RuntimeFunction,
        object: &TypedExpr,
        span: Span,
    ) -> Lowered {
        let object = self.expression(object);
        let result = self.call_runtime(function, vec![object.value], span);
        self.release(object, span);
        result
    }

    /// Makes an object of class `ty`, every byte zero, after evaluating `arguments`, then
    /// gives a scalar its value and an array its items in order; an item past the end raises
    /// `IndexError`, as CPython's does.
    fn new_cdata(&mut self, ty: &Type, arguments: &[TypedExpr], span: Span) -> Lowered {
        let class = class_of(ty);
        let values: Vec<Lowered> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let address = self.class_address(class, span);
        let object = self.call_runtime(RuntimeFunction::CdataNew, vec![address], span);
        match class {
            CData::Scalar(scalar) => {
                for value in values {
                    self.write_scalar(object.value, *scalar, value, span);
                }
            }
            CData::Array(..) => {
                let scalar = item_scalar(ty);
                for (position, value) in values.into_iter().enumerate() {
                    let index = self.constant(Constant::Int(position.to_string()), span);
                    let item = self.call_runtime(
                        RuntimeFunction::CdataItem,
                        vec![object.value, index],
                        span,
                    );
                    self.write_scalar(item.value, scalar, value, span);
                    self.release(item, span);
                }
            }
            CData::Pointer(_) => {}
        }
        object
    }

    /// Returns the value of item `index` of the pointer or array `object`, whose items are of
    /// scalar type `scalar`.
    pub(crate) fn cdata_item_value(
        &mut self,
        object: typy_tyir::ValueId,
        index: typy_tyir::ValueId,
        scalar: CType,
        span: Span,
    ) -> Lowered {
        let item = self.call_runtime(RuntimeFunction::CdataItem, vec![object, index], span);
        let value = self.read_scalar(item.value, scalar, span);
        self.release(item, span);
        value
    }

    /// Returns the value of the scalar `object` of type `scalar`, as `ctypes` gives it.
    fn read_scalar(&mut self, object: typy_tyir::ValueId, scalar: CType, span: Span) -> Lowered {
        let function = match scalar {
            CType::Float | CType::Double => RuntimeFunction::CdataGetFloat,
            CType::Bool => RuntimeFunction::CdataGetBool,
            CType::CharPointer => RuntimeFunction::CdataGetBytes,
            CType::VoidPointer => RuntimeFunction::CdataGetAddress,
            _ => RuntimeFunction::CdataGetInt,
        };
        self.call_runtime(function, vec![object], span)
    }

    /// Writes `value` into the scalar `object` of type `scalar`, converted as `ctypes`
    /// converts it, and releases it.
    fn write_scalar(
        &mut self,
        object: typy_tyir::ValueId,
        scalar: CType,
        value: Lowered,
        span: Span,
    ) {
        let (function, converted) = match scalar {
            CType::Float | CType::Double => {
                (RuntimeFunction::CdataSetFloat, self.float_of(value, span))
            }
            CType::Bool => {
                let truth = self.truth(value, span);
                (
                    RuntimeFunction::CdataSetBool,
                    Self::borrowed(truth, TyirType::Bool),
                )
            }
            CType::CharPointer => {
                let ty = TyirType::Optional(OptionalBase::Object(typy_tyir::TypeId::BYTES));
                (
                    RuntimeFunction::CdataSetBytes,
                    self.optional_of(value, ty, span),
                )
            }
            CType::VoidPointer => {
                let ty = TyirType::Optional(OptionalBase::Int);
                let number = if value.ty == TyirType::Bool {
                    self.int_of(value, span)
                } else {
                    value
                };
                (
                    RuntimeFunction::CdataSetAddress,
                    self.optional_of(number, ty, span),
                )
            }
            _ => (RuntimeFunction::CdataSetInt, self.int_of(value, span)),
        };
        self.call_runtime(function, vec![object, converted.value], span);
        // A conversion may give the value itself back; each is released once.
        if converted.value != value.value {
            self.release(converted, span);
        }
        self.release(value, span);
    }

    /// Returns `value` as the optional type `ty`: `None` of it for `None`, the value borrowed
    /// as it is when it already is one, and the value wrapped otherwise, borrowing it.
    fn optional_of(&mut self, value: Lowered, ty: TyirType, span: Span) -> Lowered {
        if value.ty == TyirType::Unit {
            return Self::borrowed(self.none_of(ty, span), ty);
        }
        if value.ty == ty {
            return Self::borrowed(value.value, ty);
        }
        let wrapped = self.builder.push_value(
            typy_tyir::Operation::Wrap {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        Self::borrowed(wrapped, ty)
    }

    /// Stores `value` into the value or an item of `object`, evaluated after it, as CPython
    /// does; the value is released.
    pub(crate) fn assign_cdata(
        &mut self,
        object: &TypedExpr,
        place: &CDataPlace,
        value: Lowered,
        span: Span,
    ) {
        let scalar = scalar_of(&object.ty, matches!(place, CDataPlace::Item(_)));
        let object = self.expression(object);
        match place {
            CDataPlace::Value => self.write_scalar(object.value, scalar, value, span),
            CDataPlace::Item(index) => {
                let index = self.expression(index);
                let item = self.call_runtime(
                    RuntimeFunction::CdataItem,
                    vec![object.value, index.value],
                    span,
                );
                self.write_scalar(item.value, scalar, value, span);
                self.release(item, span);
                self.release(index, span);
            }
        }
        self.release(object, span);
    }
}
