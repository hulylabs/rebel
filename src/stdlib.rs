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

/// Native Function of The Standard Library for the Rebel VM.
pub const NATIVES: &[NativeDescriptor] =
    &[NativeDescriptor::new("add", "Add two integers", add, 2)];
