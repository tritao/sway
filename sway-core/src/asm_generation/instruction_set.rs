use crate::asm_lang::allocated_ops::AllocatedOp;
use riscv_isa::Instruction;
use std::fmt;

#[derive(Clone)]
pub enum RiscVOp {
    Instruction(Instruction),
    Raw(String),
}

impl fmt::Display for RiscVOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiscVOp::Instruction(instruction) => write!(f, "    {}", instruction),
            RiscVOp::Raw(text) => write!(f, "{text}"),
        }
    }
}

/// An [InstructionSet] is produced by allocating registers on an [AbstractInstructionSet].
#[derive(Clone)]
pub enum InstructionSet {
    Fuel { ops: Vec<AllocatedOp> },
    Evm { ops: Vec<etk_asm::ops::AbstractOp> },
    RiscV { ops: Vec<RiscVOp> },
}

impl fmt::Display for InstructionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            ".program:\n{}",
            match self {
                InstructionSet::Fuel { ops } => ops
                    .iter()
                    .map(|x| format!("{x}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                InstructionSet::Evm { ops } => ops
                    .iter()
                    .map(|x| format!("{x}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                InstructionSet::RiscV { ops } => ops
                    .iter()
                    .map(|x| format!("{x}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            }
        )
    }
}
