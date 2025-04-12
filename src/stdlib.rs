// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{Func, Value};
use crate::vm::{NativeDescriptor, Process, VmError};

fn add(process: &mut Process) -> Result<(), VmError> {
    let [va, vb] = process.get_stack_mut().pop_n()?;
    let a = va.as_int()?;
    let b = vb.as_int()?;
    let result = a.checked_add(b).ok_or(VmError::IntegerOverflow)?;
    process
        .get_stack_mut()
        .push(Value::int(result))
        .map_err(Into::into)
}

fn lt(process: &mut Process) -> Result<(), VmError> {
    let [va, vb] = process.get_stack_mut().pop_n()?;
    let a = va.as_int()?;
    let b = vb.as_int()?;
    process
        .get_stack_mut()
        .push(Value::bool(a < b))
        .map_err(Into::into)
}

fn either(process: &mut Process) -> Result<(), VmError> {
    let [cond, if_true, if_false] = process.get_stack_mut().pop_n()?;
    let cond = cond.as_bool()?;
    let block = if cond { if_true } else { if_false };
    let series = block.as_block()?;
    let bindings = process.get_binding(series)?;
    process.call(bindings)?;
    Ok(())
}

fn func(process: &mut Process) -> Result<(), VmError> {
    let [spec, body] = process.get_stack_mut().pop_n()?;
    let spec_block = spec.as_block()?;
    let body_block = body.as_block()?;

    let arity = process.memory().len(spec_block)?;
    let func = process
        .memory_mut()
        .alloc_struct(Func::new(arity, body_block))?;

    process
        .get_stack_mut()
        .push(Value::func(func))
        .map_err(Into::into)
}

/// Native Function of The Standard Library for the Rebel VM.
pub const NATIVES: &[NativeDescriptor] = &[
    NativeDescriptor::new("add", "add two numbers function", add, 2),
    NativeDescriptor::new_op("+", "add two numbers operator", add, 1, 2),
    NativeDescriptor::new("lt", "less than function", lt, 2),
    NativeDescriptor::new_op("<", "less than operator", lt, 1, 2),
    NativeDescriptor::new("either", "execute one of two blocks", either, 3),
    NativeDescriptor::new("func", "create a function", func, 2),
];
