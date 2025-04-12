// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::Value;
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

fn either(process: &mut Process) -> Result<(), VmError> {
    let [cond, if_true, if_false] = process.get_stack_mut().pop_n()?;
    let cond = cond.as_bool()?;
    let block = if cond { if_true } else { if_false };
    let series = block.as_block()?;
    let bindings = process.get_binding(series)?;
    process.call(bindings)?;
    Ok(())
}

/// Native Function of The Standard Library for the Rebel VM.
pub const NATIVES: &[NativeDescriptor] = &[
    NativeDescriptor::new("add", "add two numbers", add, 2),
    NativeDescriptor::new_op("+", "add two numbers operator", add, 1, 2),
    NativeDescriptor::new("either", "execute one of two blocks", either, 3),
];
