//! Emits the three functions through which the runtime allocates, resizes and frees memory,
//! each calling the function the project names, or the C library's (MEM-08, ADR-0043).

use inkwell::AddressSpace;
use inkwell::GlobalVisibility;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::FunctionType;
use inkwell::values::BasicMetadataValueEnum;
use typy_tyir::TyirAllocator;

use crate::error::CodegenError;

/// The names the runtime calls the hooks by.
pub(crate) const ALLOCATE: &str = "typy_rt_heap_allocate";
pub(crate) const REALLOCATE: &str = "typy_rt_heap_reallocate";
pub(crate) const FREE: &str = "typy_rt_heap_free";

/// Defines the hooks in `module`. They are hidden, so a shared library does not export them,
/// and each calls its function in tail position with the C signature of `malloc`, `realloc`
/// or `free`.
pub(crate) fn emit_heap_hooks<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    allocator: &TyirAllocator,
) -> Result<(), CodegenError> {
    let pointer = context.ptr_type(AddressSpace::default());
    let size = context.i64_type();
    let hooks = [
        (
            ALLOCATE,
            allocator.allocate.as_str(),
            pointer.fn_type(&[size.into()], false),
        ),
        (
            REALLOCATE,
            allocator.reallocate.as_str(),
            pointer.fn_type(&[pointer.into(), size.into()], false),
        ),
        (
            FREE,
            allocator.free.as_str(),
            context.void_type().fn_type(&[pointer.into()], false),
        ),
    ];
    for (hook, target, ty) in hooks {
        emit_hook(context, module, hook, target, ty)?;
    }
    Ok(())
}

fn emit_hook<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    hook: &str,
    target: &str,
    ty: FunctionType<'ctx>,
) -> Result<(), CodegenError> {
    let function = module.add_function(hook, ty, Some(Linkage::External));
    function
        .as_global_value()
        .set_visibility(GlobalVisibility::Hidden);
    // A target the module already has with another type, such as one of the program's own
    // functions, is called through its address with the hook's type, which LLVM accepts.
    let callee = module
        .get_function(target)
        .unwrap_or_else(|| module.add_function(target, ty, Some(Linkage::External)));
    let builder = context.create_builder();
    builder.position_at_end(context.append_basic_block(function, ""));
    let arguments: Vec<BasicMetadataValueEnum<'ctx>> =
        function.get_param_iter().map(Into::into).collect();
    let call = builder.build_indirect_call(
        ty,
        callee.as_global_value().as_pointer_value(),
        &arguments,
        "",
    )?;
    call.set_tail_call(true);
    match call.try_as_basic_value().basic() {
        Some(result) => builder.build_return(Some(&result))?,
        None => builder.build_return(None)?,
    };
    Ok(())
}
