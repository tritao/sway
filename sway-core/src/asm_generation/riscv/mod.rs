use std::collections::HashMap;

use super::{
    asm_builder::AsmBuilder,
    fuel::data_section::DataSection,
    instruction_set::{InstructionSet, RiscVOp},
    ProgramKind,
};
use crate::{asm_lang::Label, BuildConfig};
use riscv_isa::Instruction;
use sway_error::handler::{ErrorEmitted, Handler};
use sway_ir::{ConfigContent, Context, Function};

#[derive(Default)]
struct RiscVEntry {
    fn_name: String,
    selector: Option<[u8; 4]>,
}

pub struct RiscVAsmBuilder<'ir> {
    kind: ProgramKind,
    context: &'ir Context<'ir>,
    next_label: usize,
    func_label_map: HashMap<Function, (Label, Label)>,
    ops: Vec<RiscVOp>,
    entries: Vec<RiscVEntry>,
}

impl<'ir> RiscVAsmBuilder<'ir> {
    pub fn new(kind: ProgramKind, context: &'ir Context<'ir>) -> Self {
        Self {
            kind,
            context,
            next_label: 0,
            func_label_map: HashMap::new(),
            ops: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn fresh_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }

    fn push_raw_line<S: Into<String>>(&mut self, line: S) {
        self.ops.push(RiscVOp::Raw(line.into()));
    }

    fn push_instruction(&mut self, instruction: Instruction) {
        self.ops.push(RiscVOp::Instruction(instruction));
    }
}

impl<'ir> AsmBuilder for RiscVAsmBuilder<'ir> {
    fn func_to_labels(&mut self, func: &Function) -> (Label, Label) {
        if let Some(labels) = self.func_label_map.get(func) {
            *labels
        } else {
            let start = self.fresh_label();
            let end = self.fresh_label();
            self.func_label_map.insert(*func, (start, end));
            (start, end)
        }
    }

    fn compile_configurable(&mut self, config: &ConfigContent) {
        // Reserve a placeholder comment for configurables to aid debugging.
        let name = match config {
            ConfigContent::V0 { name, .. } => name.clone(),
            ConfigContent::V1 { name, .. } => name.clone(),
        };
        self.push_raw_line(format!("; configurable {name} (not lowered yet)"));
    }

    fn compile_function(
        &mut self,
        _handler: &Handler,
        function: Function,
    ) -> Result<(), ErrorEmitted> {
        let fn_name = function.get_name(self.context).to_string();

        self.push_raw_line(format!("; begin function {fn_name}"));
        self.push_raw_line(format!("{fn_name}:"));
        self.push_raw_line("    ; TODO: lower function body");
        self.push_instruction(Instruction::JALR {
            rd: 0,
            rs1: 1,
            offset: 0,
        });
        self.push_raw_line(format!("; end function {fn_name}"));
        self.push_raw_line("");

        if function.is_entry(self.context) {
            self.entries.push(RiscVEntry {
                fn_name,
                selector: function.get_selector(self.context),
            });
        }

        Ok(())
    }

    fn finalize(
        self,
        _handler: &Handler,
        _build_config: Option<&BuildConfig>,
        _fallback_fn: Option<Label>,
    ) -> Result<super::FinalizedAsm, ErrorEmitted> {
        let entries = self
            .entries
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| super::FinalizedEntry {
                fn_name: entry.fn_name,
                imm: idx as u64,
                selector: entry.selector,
                test_decl_ref: None,
            })
            .collect();

        Ok(super::FinalizedAsm {
            data_section: DataSection::default(),
            program_section: InstructionSet::RiscV { ops: self.ops },
            program_kind: self.kind,
            entries,
            abi: None,
        })
    }
}
