# Sway RISC-V Backend — Implementation Plan

## Scope and Principles
- Target the RVA22U64 profile first; keep XLEN-parameterized hooks for RV32/RV128.
- Maintain Fuel/EVM stability via feature flags and progressive integration.
- Drive each increment with automated tests and runnable examples.

## Stage 1: Backend Skeleton MVP
- **Objective**: Allow `BuildTarget::RiscV` to flow through configuration and emit placeholder RISC-V assembly for arithmetic/control flow programs.
- **Primary Tasks**: extend build configuration, dispatch to a stub `RiscVAsmBuilder`, emit minimal textual assembly, gate behind a feature flag, document usage.
- **Validation**: unit tests for target selection (`sway-core/src/build_config.rs`), integration test compiling a trivial contract that compares emitted assembly to a golden snapshot.
- **Exit Criteria**: CI passes with the feature flag on; docs show how to enable the experimental target.

## Stage 2: Intrinsic Refactor (Feature-Flagged)
- **Objective**: Introduce a target-neutral intrinsic layer without regressing Fuel/EVM.
- **Primary Tasks**: create `TargetIntrinsic`, migrate Fuel intrinsics, stub `RiscVIntrinsic`, update IR passes to switch on the new enum while a feature flag keeps legacy paths available.
- **Validation**: dual-run existing IR lowering tests under both flag states; targeted unit tests ensuring correct pattern matching for new intrinsic variants.
- **Exit Criteria**: Fuel/EVM builds unchanged with flag off; new tests green with flag on.

## Stage 3: Legalization Groundwork
- **Objective**: Lower IR plus intrinsics into RISC-V oriented pseudo operations parameterized by XLEN.
- **Primary Tasks**: implement legalization pass for arithmetic, branching, calls; ensure type sizing and pointer math respect configuration; generate pseudo ops recorded in IR metadata.
- **Validation**: property tests around sign/zero extension and pointer arithmetic; golden files snapshotting pseudo op sequences for representative functions.
- **Exit Criteria**: Legalization pass stable under optimization pipelines; pseudo IR snapshots committed and reviewed.

## Stage 4: `RiscVAsmBuilder` Bring-Up
- **Objective**: Translate pseudo ops into concrete RV64GCB instructions with basic register allocation.
- **Primary Tasks**: implement linear-scan allocator, stack frame management, constant materialization, instruction selection for RV64I/M/B/C subsets; emit deterministic assembly.
- **Validation**: unit tests covering allocator edge cases and frame layout; integration tests compiling sample Sway programs and asserting assembly fragments.
- **Exit Criteria**: Backend emits runnable assembly for control flow and arithmetic samples; lint/format checks pass.

## Stage 5: Host Intrinsic Lowering
- **Objective**: Support storage/logging/transaction intrinsics via `ecall` conventions and runtime helpers.
- **Primary Tasks**: reserve `t6` for environment pointer, map intrinsics to syscall IDs, add runtime stubs for wide math and memory routines, define ABI documentation within the repo.
- **Validation**: mock host ABI tests checking syscall IDs and argument marshalling; compile-time assertions that reserved registers stay untouched; doc tests for ABI snippets.
- **Exit Criteria**: Feature-flagged programs can invoke host calls without panics; ABI reference merged.

## Stage 6: Tooling Integration
- **Objective**: Wire `forc` and packaging to produce RISC-V artifacts.
- **Primary Tasks**: accept `--target riscv`, emit assembly/object/metadata via assembler backend or crate, add `ProgramABI::RiscV` variant, forward target info through manifests and metadata.
- **Validation**: CLI regression tests verifying artifact layout; unit tests covering ABI serialization; snapshot tests to ensure deterministic metadata.
- **Exit Criteria**: Developers can run `forc build --target riscv` (flagged) end-to-end; CI jobs updated to exercise the new path.

## Stage 7: Emulator Harness
- **Objective**: Execute compiled artifacts inside `libriscv` interpreter mode with deterministic host hooks.
- **Primary Tasks**: integrate libriscv, register syscall handlers (`0x100–0x10F`), implement gas metering shims, enforce memory bounds, provide developer runner tooling.
- **Validation**: e2e tests running contracts to completion (success, revert, storage interactions), determinism checks across runs/platforms, gas budget regression tests.
- **Exit Criteria**: Harness produces consistent outputs and emits telemetry for logs/gas; incorporated into CI e2e suite.

## Stage 8: Feature Expansion and Stabilization
- **Objective**: Add advanced intrinsics, harden ABI semantics, and prepare for GA.
- **Primary Tasks**: implement storage batching, messaging, performance tuning, expand documentation, prepare RV32/RV128 gating hooks.
- **Validation**: per-feature e2e scenarios, regression suite for gas accounting and error propagation, doc review.
- **Exit Criteria**: Feature flag removed after coverage and stakeholder sign-off; roadmap for follow-on work published.
