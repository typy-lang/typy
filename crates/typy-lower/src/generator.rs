//! Lowers generator functions (SMT-11, FUN-07, ADR-0063). A generator function becomes two
//! functions: the one its call runs, which makes the generator and stores each argument in
//! its frame, and its resume function, which takes the generator, keeps its slots in the
//! frame, and starts each step at the block after the `yield` it stopped at.

use typy_check::{TypedExpr, TypedFunction, TypedProgram};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, FunctionBuilder, FunctionId, GeneratorFrame, Operation, Terminator, TupleTypeId,
    TupleTypes, TyirFunction, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::mangle::symbol;
use crate::types::tyir_type;

/// What the lowering of a resume function keeps: its generator, its ID, the block of its
/// first step, the block after each `yield`, in order, and what its frame keeps.
pub(crate) struct Resumable {
    generator: ValueId,
    function: FunctionId,
    /// The block of the first step, which the prologue fills.
    pub(crate) start: BlockId,
    points: Vec<BlockId>,
    /// The types of the arguments the frame keeps, the closure first when there is one.
    arguments: Vec<TyirType>,
    closure: bool,
    yields: TyirType,
}

/// Where the first step of a resume function finds its parameters: its generator, its ID,
/// and the position of the first parameter among the arguments, after the closure.
#[derive(Clone, Copy)]
pub(crate) struct Arguments {
    generator: ValueId,
    function: FunctionId,
    first: u32,
}

impl Arguments {
    /// Returns parameter `position`, of type `ty`, taken out of the frame with its reference.
    pub(crate) fn take(
        self,
        builder: &mut FunctionBuilder,
        position: usize,
        ty: TyirType,
        span: Span,
    ) -> ValueId {
        let index = self.first + u32::try_from(position).unwrap_or(u32::MAX);
        builder.push_value(
            Operation::GeneratorArgument {
                generator: self.generator,
                function: self.function,
                index,
            },
            ty,
            span,
        )
    }
}

impl Resumable {
    /// Returns where the first step finds the parameters.
    pub(crate) fn arguments(&self) -> Arguments {
        Arguments {
            generator: self.generator,
            function: self.function,
            first: u32::from(self.closure),
        }
    }
}

/// Starts the resume function `resume` of the generator function `function`: it takes the
/// generator, reads the closure of a nested one at every step, before its entry chooses where
/// the step starts, and continues in the block of the first step. Returns the builder there,
/// the lowering's state, and the closure with the layout of its captures.
pub(crate) fn resume_builder(
    program: &TypedProgram,
    function: &TypedFunction,
    resume: FunctionId,
    tuple_types: &mut TupleTypes,
) -> (
    FunctionBuilder,
    Option<Resumable>,
    Option<(ValueId, TupleTypeId)>,
) {
    let name = symbol(
        &function.module,
        &format!("{}.<generator>", function.qualified_name),
    );
    let mut builder = FunctionBuilder::new(resume, name, function.name.clone(), TyirType::Unit);
    let generator = builder.add_parameter(TyirType::GENERATOR);
    let closure = program.closures.get(&function.id).map(|captures| {
        let layout = crate::function::captures_layout(captures, tuple_types);
        let value = builder.push_value(
            Operation::GeneratorClosure {
                generator,
                function: resume,
            },
            TyirType::FUNCTION,
            function.span,
        );
        (value, layout)
    });
    let start = builder.create_block();
    builder.switch_to(start);
    let mut arguments: Vec<TyirType> = closure.iter().map(|_| TyirType::FUNCTION).collect();
    arguments.extend(
        function
            .parameters
            .iter()
            .map(|parameter| tyir_type(&program.symbol(*parameter).ty, tuple_types)),
    );
    let yields = function.generator.as_ref().map_or(TyirType::Unit, |types| {
        tyir_type(&types.yields, tuple_types)
    });
    let resumable = Resumable {
        generator,
        function: resume,
        start,
        points: Vec::new(),
        arguments,
        closure: closure.is_some(),
        yields,
    };
    (builder, Some(resumable), closure)
}

/// Returns the function a call of the generator function `function` runs: it makes a
/// generator of the resume function `resume`, stores each argument in its frame with a
/// reference of its own, the closure of a nested one first, and returns the generator as the
/// annotation's type, the same object (ADR-0063).
pub(crate) fn maker(
    program: &TypedProgram,
    function: &TypedFunction,
    resume: FunctionId,
    tuple_types: &mut TupleTypes,
) -> TyirFunction {
    let return_type = tyir_type(&function.signature.returns, tuple_types);
    let mut builder = FunctionBuilder::new(
        FunctionId(function.id.0),
        crate::function::function_symbol(program, function),
        function.name.clone(),
        return_type,
    );
    builder.set_qualified_name(function.qualified_name.clone());
    let span = function.span;
    let mut arguments = Vec::new();
    if program.closures.contains_key(&function.id) {
        let closure = builder.add_parameter(TyirType::FUNCTION);
        arguments.push((closure, TyirType::FUNCTION));
    }
    for parameter in &function.parameters {
        let ty = tyir_type(&program.symbol(*parameter).ty, tuple_types);
        arguments.push((builder.add_parameter(ty), ty));
    }
    let generator = builder.push_value(
        Operation::NewGenerator { function: resume },
        TyirType::GENERATOR,
        span,
    );
    for (index, (value, ty)) in arguments.iter().enumerate() {
        if tuple_types.is_counted(*ty) {
            builder.push(Operation::Increment(*value), span);
        }
        builder.push(
            Operation::SetGeneratorArgument {
                generator,
                function: resume,
                index: u32::try_from(index).unwrap_or(u32::MAX),
                value: *value,
            },
            span,
        );
    }
    let result = match return_type {
        TyirType::GENERATOR => generator,
        TyirType::ITERATOR => builder.push_value(
            Operation::Upcast {
                value: generator,
                ty: TyirType::ITERATOR,
            },
            TyirType::ITERATOR,
            span,
        ),
        // An iterable is the same object, so only its type changes.
        _ => {
            let bits = builder.push_value(Operation::ToBits(generator), TyirType::Bits, span);
            builder.push_value(
                Operation::FromBits {
                    bits,
                    ty: return_type,
                },
                return_type,
                span,
            )
        }
    };
    builder.terminate(Terminator::Return(result));
    builder.finish()
}

impl FunctionLowerer<'_> {
    /// Lowers `yield value`: the generator takes over a reference to the value as its current
    /// item, notes where the next step continues, and ends the step there.
    pub(crate) fn yield_value(&mut self, value: &TypedExpr, span: Span) {
        let lowered = self.expression(value);
        let owned = self.take_reference(lowered, span);
        let item = self
            .builder
            .push_value(Operation::ToBits(owned), TyirType::Bits, span);
        let resume = self.builder.create_block();
        let Some(resumable) = self.generator.as_mut() else {
            unreachable!("the checker gives `yield` to generator functions alone")
        };
        resumable.points.push(resume);
        let generator = resumable.generator;
        self.builder
            .push(Operation::SetGeneratorItem { generator, item }, span);
        self.builder
            .terminate(Terminator::Yield { generator, resume });
        self.builder.switch_to(resume);
    }

    /// Ends a resume function: its entry starts each step in the block of the first step or
    /// after the yield it stopped at, and its frame holds the arguments, then every slot.
    pub(crate) fn finish_resume(mut self) -> TyirFunction {
        let Some(resumable) = self.generator.take() else {
            unreachable!("only a resume function finishes as one")
        };
        self.builder.switch_to(BlockId(0));
        self.builder.terminate(Terminator::Resume {
            generator: resumable.generator,
            start: resumable.start,
            points: resumable.points,
        });
        let mut items = resumable.arguments.clone();
        items.extend(self.builder.slot_types());
        let frame = self.tuple_types.intern_id(items);
        self.builder.set_generator(GeneratorFrame {
            arguments: resumable.arguments,
            closure: resumable.closure,
            yields: resumable.yields,
            frame,
        });
        // The runtime calls every resume function alike, so each returns as one that raises.
        self.builder.set_may_raise(true);
        self.builder.finish()
    }
}
