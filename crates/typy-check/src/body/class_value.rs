//! Reads and assigns the values a class body gives: class variables and the defaults of
//! instance attributes, each held as the module variable `Class.name` of the class's module
//! (CLS-11, ADR-0024).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier};

use super::BodyChecker;
use crate::typed::{AssignTarget, ClassId, SymbolId, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

/// What an assignment to a value a class body gives comes to.
pub(super) enum ClassValueAssignment {
    /// The module variable that holds the value is assigned.
    Assigned(AssignTarget),
    /// The assignment is refused, and reported.
    Refused,
}

/// Where a class's value is held: the class whose body gives it, and whether it is the
/// default of an instance attribute rather than a class variable.
#[derive(Clone, Copy)]
struct Holder {
    class: ClassId,
    default: bool,
}

impl BodyChecker<'_> {
    /// Returns the class a type object stands for, when it is a class of the program.
    pub(super) fn program_class(of: &TypeObjectOf) -> Option<ClassId> {
        match of {
            TypeObjectOf::Instance(class, _) => Some(*class),
            TypeObjectOf::Exception(exception) => exception.class_id(),
            _ => None,
        }
    }

    /// Returns true when the local `symbol` is `cls` of the class method being checked, which
    /// holds its class exactly, since each class has its own copy of the method (ADR-0025).
    pub(super) fn is_class_method_class(&self, symbol: SymbolId) -> bool {
        self.class_parameter == Some(symbol)
    }

    /// Returns true when `value`, a type object of `class` or of a class derived from it, may
    /// hold a derived class: it is neither the class's name nor `cls` of a class method, and
    /// a class of the program derives from `class` (CLS-39, ADR-0084).
    pub(super) fn may_hold_derived(&self, value: &TypedExpr, class: ClassId) -> bool {
        let exact = match &value.kind {
            TypedExprKind::TypeObject(_) => true,
            TypedExprKind::Local(symbol) => self.is_class_method_class(*symbol),
            _ => false,
        };
        !exact && self.program.has_derived_class(class)
    }

    /// Returns the refusal of reading `value.member` as a value: a member of a named tuple
    /// planned for 0.14 (CLS-43), or one reached through a class that may be derived (CLS-39).
    pub(super) fn refused_member(
        &mut self,
        value: &TypedExpr,
        member: &Identifier,
    ) -> Option<TypedExpr> {
        self.planned_member(&value.ty, member, false)
            .or_else(|| self.member_through_held_class(value, member))
    }

    /// Returns the refusal of `value.member` where `value` is a `type[C]` that may hold a
    /// class derived from `C` and the member is a class method, a class variable or a default
    /// a derived class may give again, planned for 0.14 (CLS-39); `None` for any other read.
    pub(super) fn member_through_held_class(
        &mut self,
        value: &TypedExpr,
        member: &Identifier,
    ) -> Option<TypedExpr> {
        let Type::TypeObject(Some(of)) = &value.ty else {
            return None;
        };
        let class = Self::program_class(of)?;
        let reaches =
            self.has_method(class, &member.name) || self.holder(class, &member.name).is_some();
        if !reaches || !self.may_hold_derived(value, class) {
            return None;
        }
        Some(self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                member.span,
                format!(
                    "`{}` through a `{}` value that may hold a class derived from it is planned \
                     for TyPy 0.14 (CLS-39)",
                    member.name, value.ty
                ),
            )
            .with_rule(RuleId::CLS_39)
            .with_help("name the class, or test the value with `issubclass` first"),
            member.span,
        ))
    }

    /// Returns the class an expression names: a class's name, or a variable that holds a
    /// class's type object, such as `cls` in a class method (ADR-0025).
    pub(super) fn class_named_by(&self, object: &Expr) -> Option<ClassId> {
        let Expr::Name { id, .. } = object else {
            return None;
        };
        match self.locals.get(id) {
            None => self.program.class_ids.get(id).copied(),
            Some(symbol) => match self.types.get(symbol)? {
                Type::TypeObject(Some(of)) => Self::program_class(of),
                _ => None,
            },
        }
    }

    /// Returns the class whose body gives `class`, or the nearest base, a value `name`.
    fn holder(&self, class: ClassId, name: &str) -> Option<Holder> {
        let mut current = Some(class);
        while let Some(each) = current {
            let info = &self.program.classes[each.0 as usize];
            if info.class_variables.iter().any(|variable| variable == name) {
                return Some(Holder {
                    class: each,
                    default: false,
                });
            }
            if info
                .attribute(name)
                .is_some_and(|index| info.defaults.contains(&index))
            {
                return Some(Holder {
                    class: each,
                    default: true,
                });
            }
            current = info.base;
        }
        None
    }

    /// Returns the positions of the attributes of `class` that its body, or a base's, gives a
    /// default, which each new instance holds before `__init__` runs.
    pub(super) fn defaults(&self, class: ClassId) -> Vec<usize> {
        let mut found = Vec::new();
        let mut current = Some(class);
        while let Some(each) = current {
            let info = &self.program.classes[each.0 as usize];
            found.extend(info.defaults.iter().copied());
            current = info.base;
        }
        found
    }

    /// Returns the name of the module variable that holds a holder's value, with the module.
    fn holder_variable(&self, holder: Holder, name: &str) -> (String, String) {
        let info = &self.program.classes[holder.class.0 as usize];
        (
            crate::check::class_value_variable(&info.name, name),
            self.program.class_modules[holder.class.0 as usize].clone(),
        )
    }

    /// Reads the value `name` the body of `class`, or of a base, gives, when it gives one:
    /// `Class.name` for a class variable or a default, and `instance.name` for a class
    /// variable. An instance holds each attribute with a default, so reading one through an
    /// instance reads the attribute and never reaches here.
    pub(super) fn class_value_read(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        let holder = self.holder(class, &attribute.name)?;
        // A subclass may hold the variable of its own, which a read tests first (ADR-0037).
        if !holder.default {
            let chain = self
                .program
                .variable_chain(class, holder.class, &attribute.name);
            if chain.len() > 1 {
                return Some(self.program.chain_read(chain, span));
            }
        }
        let (variable, module) = self.holder_variable(holder, &attribute.name);
        Some(if module == self.program.module {
            self.expression(&Expr::Name { id: variable, span })
        } else {
            self.module_attribute(&module, &variable, span)
        })
    }

    /// Returns `instance.name` of a class variable some class derived from its class holds
    /// one of its own of: a call of the getter of the instance's class, through the method
    /// table (ADR-0037).
    pub(super) fn shared_class_variable(
        &mut self,
        object: &TypedExpr,
        class: ClassId,
        attribute: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        let holder = self
            .holder(class, &attribute.name)
            .filter(|holder| !holder.default)?;
        if !self.program.shares(holder.class, &attribute.name) {
            return None;
        }
        let getter_name = super::class_variable_chain::getter_name(&attribute.name);
        let getter = self.program.method(class, &getter_name)?;
        let returns = self.program.signatures[getter.0 as usize].returns.clone();
        Some(Self::typed(
            crate::typed::TypedExprKind::Call {
                function: getter,
                arguments: vec![crate::typed::Argument {
                    passed: crate::typed::Passed::Position,
                    parameter: 0,
                    value: object.clone(),
                }],
                dispatch: true,
            },
            returns,
            span,
        ))
    }

    /// Checks `Class.name = value` for a class variable, which assigns the module variable
    /// that holds it. A default of an instance attribute is read through the class only,
    /// since instances that never assigned the attribute would read a new value in CPython.
    pub(super) fn class_value_target(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
        ty: &Type,
        value: &TypedExpr,
        span: Span,
    ) -> Option<ClassValueAssignment> {
        let holder = self.holder(class, &attribute.name)?;
        let class_name = self.program.classes[class.0 as usize].name.clone();
        if holder.default {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    span,
                    format!(
                        "`{class_name}.{}` is the default of an instance attribute, which an \
                         instance assigns",
                        attribute.name
                    ),
                )
                .with_rule(RuleId::CLS_11)
                .with_help("declare it `ClassVar` to share one value through the class"),
            );
            return Some(ClassValueAssignment::Refused);
        }
        // A subclass the program assigns the variable through holds one of its own.
        let holder = if self.program.holds_own(class, &attribute.name) {
            self.program
                .variable_chain(class, holder.class, &attribute.name);
            Holder {
                class,
                default: false,
            }
        } else {
            holder
        };
        if let Some(refused) = self.inherited_class_variable(class, holder, attribute, span) {
            return Some(refused);
        }
        let (variable, module) = self.holder_variable(holder, &attribute.name);
        if module != self.program.module {
            let target = Identifier {
                name: variable,
                span: attribute.span,
            };
            return Some(
                self.module_attribute_target(&module, &target, ty, value, span)
                    .map_or(
                        ClassValueAssignment::Refused,
                        ClassValueAssignment::Assigned,
                    ),
            );
        }
        // The class's module variable is assigned from any function, as if declared `global`;
        // no Python name has a dot, so it never hides a local.
        if !self.locals.contains_key(&variable)
            && let Some(symbol) = self.program.module_symbols.get(&variable).copied()
        {
            let declared = self.program.symbols[symbol.0 as usize].ty.clone();
            self.global_names.insert(variable.clone());
            self.locals.insert(variable.clone(), symbol);
            self.declared.insert(symbol, declared.clone());
            self.types.insert(symbol, declared);
        }
        let target = Expr::Name { id: variable, span };
        Some(
            self.assign(&target, ty.clone(), value)
                .map_or(ClassValueAssignment::Refused, |symbol| {
                    ClassValueAssignment::Assigned(AssignTarget::Name(symbol))
                }),
        )
    }

    /// Reports `instance.name = value` for a class variable, which in CPython gives that
    /// instance a value of its own.
    pub(super) fn class_variable_through_instance(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
        span: Span,
    ) -> bool {
        let Some(holder) = self
            .holder(class, &attribute.name)
            .filter(|holder| !holder.default)
        else {
            return false;
        };
        let owner = self.program.classes[holder.class.0 as usize].name.clone();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(414),
                span,
                format!(
                    "`{}` is a class variable of `{owner}`, which an instance does not assign",
                    attribute.name
                ),
            )
            .with_rule(RuleId::CLS_11)
            .with_help(format!("assign `{owner}.{}`", attribute.name)),
        );
        true
    }

    /// Checks `Class.name op= value` for a class variable, which reads and assigns the module
    /// variable that holds it; `None` when the target is not a value a class body gives.
    pub(super) fn augmented_class_value(
        &mut self,
        object: &Expr,
        attribute: &Identifier,
        current: &TypedExpr,
        ty: &Type,
        span: Span,
    ) -> Option<ClassValueAssignment> {
        let symbol = match &current.kind {
            crate::typed::TypedExprKind::ModuleVariable(symbol)
            | crate::typed::TypedExprKind::ModuleAttribute { symbol, .. }
                if self.program.symbols[symbol.0 as usize].name.contains('.') =>
            {
                *symbol
            }
            // A read down the chain of a subclass's own holder assigns that holder.
            crate::typed::TypedExprKind::ClassVariable(holders) => *holders.first()?,
            _ => return None,
        };
        let Some(class) = self.class_named_by(object) else {
            // In CPython, `instance.name op= value` gives that instance a value of its own.
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(414),
                    span,
                    format!(
                        "`{}` is a class variable, which an instance does not assign",
                        attribute.name
                    ),
                )
                .with_rule(RuleId::CLS_11)
                .with_help("assign it through its class, such as `Counter.count += 1`"),
            );
            return Some(ClassValueAssignment::Refused);
        };
        let holder = self.holder(class, &attribute.name)?;
        if holder.default {
            return self.class_value_target(class, attribute, ty, current, span);
        }
        if !self.program.holds_own(class, &attribute.name)
            && let Some(refused) = self.inherited_class_variable(class, holder, attribute, span)
        {
            return Some(refused);
        }
        let name = self.program.symbols[symbol.0 as usize].name.clone();
        if self.final_reassigned(symbol, &name, span) {
            return Some(ClassValueAssignment::Refused);
        }
        if !ty.is_assignable_to(&current.ty) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(402),
                    span,
                    format!(
                        "a class variable of type {} is assigned {}",
                        current.ty,
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_03),
            );
        }
        Some(ClassValueAssignment::Assigned(AssignTarget::Name(symbol)))
    }

    /// Reports `cls.name = value` for a class variable a base gives, through a variable that
    /// holds a class derived from it, which the scan of the program cannot see; through a
    /// class's name it gives the class a holder of its own (ADR-0037).
    fn inherited_class_variable(
        &mut self,
        class: ClassId,
        holder: Holder,
        attribute: &Identifier,
        span: Span,
    ) -> Option<ClassValueAssignment> {
        if holder.class == class {
            return None;
        }
        let class_name = self.program.classes[class.0 as usize].name.clone();
        let owner = self.program.classes[holder.class.0 as usize].name.clone();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "assigning `{}`, a class variable of `{owner}`, through a variable that \
                     holds a class derived from it, which gives that class one of its own in \
                     CPython, is planned for TyPy 0.14 (CLS-31)",
                    attribute.name
                ),
            )
            .with_rule(RuleId::CLS_31)
            .with_help(format!(
                "assign it through the class by its name, such as `{class_name}.{} = ...`",
                attribute.name
            )),
        );
        Some(ClassValueAssignment::Refused)
    }
}
