// Rebel™ © 2025 Huly Labs • https://hulylabs.com • SPDX-License-Identifier: MIT

use crate::{
    mem::MemoryError,
    vm::{ByteCode, Code, Process},
};

fn add(code: &mut ByteCode) -> Result<(), MemoryError> {
    code.push(Code::ADD)
}

pub fn load(process: &mut Process) -> Result<(), MemoryError> {
    process.add_instrinsic("add", add)?;
    Ok(())
}
