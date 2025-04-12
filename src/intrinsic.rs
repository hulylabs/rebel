// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::mem::{MemoryError, Value};
use crate::vm::{Process, VmError};

pub fn add(process: &mut Process) -> Result<(), VmError> {
    let [va, vb] = process.get_stack_mut().pop_n()?;
    let a = va.as_int()?;
    let b = vb.as_int()?;
    let result = a.checked_add(b).ok_or(VmError::IntegerOverflow)?;
    process.get_stack_mut().push(Value::int(result))?;
    Ok(())
}

pub fn load(process: &mut Process) -> Result<(), MemoryError> {
    process.add_instrinsic("add", add)?;
    Ok(())
}
