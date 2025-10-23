# Sway RISC-V Backend — Revamped Design Proposal

## Overview

This document outlines a proposal to add a first-class **RISC-V backend** to the Sway compiler toolchain. Sway programs will target the **RVA22U64** profile (RV64GCB baseline, engineered to extend to RV32GCB and experimental RV128G) via a new backend that consumes Sway IR, introduces a target-neutral intrinsic layer, and lowers to RISC-V assembly/bytecode.

Execution is provided by the **libriscv** userspace RISC-V sandbox in deterministic, interpreter-only mode.

In summary we will discuss:

* A precise explanation of RVA22U64 and the RV64GCB baseline.
    
* A **FuelVM→RISC-V parity plan** (endianness, gas, receipts, context/call-frame isolation, predicates, crypto).
    
* A concrete **host ABI** mapping from Fuel contract/crypto ops to RISC-V ecalls.
    
* The role of the **Fuel ISA** during migration (as a reference backend and oracle) and an off-ramp strategy.
    
* libriscv integration details for consensus determinism.
    

* * *

## Goals

* End-to-end path from Sway source → RISC-V bytecode (config plumbing, IR lowering, backend codegen, packaging).
    
* Deterministic **contract ABI** (transaction context, storage, logging, messaging, gas semantics).
    
* Refactor `sway-ir` intrinsics so Fuel/EVM/RISC-V share one IR with target-specific legalization.
    
* Initial support for **RVA22U64**, leaving guardrails for RV32/RV128.
    
* SDK/tooling/test-harness integration so developers can build, run, and test RISC-V Sway programs.
    

## Non-Goals

* Fuel bytecode → RISC-V translation (we lower **Sway IR** to RISC-V; FuelVM emulation is out of scope).
    
* Production-ready RV128G (experimental only; the design is extensible to it).
    

* * *

## Understanding the Target: **RVA22U64** (RV64GCB)

**Profiles** are standardized RISC-V bundles that guarantee a consistent unprivileged ISA environment across hardware.  
**RVA22U64** = Application-class, 2022 edition, unprivileged, **64-bit XLEN**.

**RV64GCB baseline** (what you can rely on):

* **G** macro = **I+M+A+F+D** (integer, mul/div, atomics, single/double FP)
    
* **B** (bit-manipulation)
    
* **C** (compressed 16-bit encodings)
    

Design is **XLEN-parameterized** for future **RV32GCB** (ILP32) and experimental **RV128G**.

* * *

## Current State Summary

* `BuildTarget` only enumerates Fuel & EVM.
    
* IR intrinsics encode Fuel semantics (`InstOp::FuelVm`).
    
* Backends implement `AsmBuilder` for Fuel/EVM; no RISC-V yet.
    
* Tooling (`forc`, tests) assumes Fuel/EVM artifacts.
    
* Type sizing/alignment tuned for FuelVM’s 64-bit words.
    

* * *

## Compatibility & Parity with FuelVM: Gaps and Mitigations

### 1) Endianness & Encoding Policy

* **FuelVM integers are big-endian**; registers/words are 64-bit. RISC-V/libriscv are **little-endian** by default.  
    **Policy:** treat all **consensus-visible encodings as explicit byte strings**; convert at **host ecall boundaries** (guest stays little-endian). Do **not** let endianness leak into on-chain formats. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

### 2) Initialization, Stack Model, Contexts & Call-Frame Ownership

* FuelVM uses a **single VM memory**, initializes with tx data, stack grows **upward**, and enforces **context/call-frame isolation with ownership rules** (unwritable areas, cross-area reads panic).  
    **Policy:** run each external/script/predicate/contract call in a **fresh libriscv instance** with isolated memory; marshal parameters/returns via host memory; enforce ownership/bounds in **ecall handlers**. This preserves isolation semantics without re-implementing Fuel’s memory checks in hardware. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

### 3) Gas Model & Instruction Accounting

* Fuel deducts gas **before** each instruction; has **global** and **context** gas registers; on insufficient gas, **revert without executing** the instruction.  
    **Policy:** add a **host step hook** that charges per **RISC-V opcode** and per **ecall**. Track **global**/**context** meters and pre-deduct; on insufficiency, revert identically for the current context. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

### 4) Receipts & Receipts Root

* Fuel emits structured receipts (including **Panic** and **ScriptResult**) and updates the tx **receipts Merkle root**; there is a receipts count ceiling.  
    **Policy:** reproduce **receipt types/fields** and **Merkle root** exactly (or define a replacement spec in lock-step with node/consensus). Enforce limits and identical panic/script-result behavior. [Fuel Docs+1](https://docs.fuel.network/docs/specs/fuel-vm/)
    

### 5) Contract/State Ops, Metadata, and Blob Support

* Fuel provides native instructions for contract calls, state read/write (32-byte slots), logging, balances, code/metadata inspection, block height/hash/time, send-message, and blob ops (**BSIZ/BLDD**).  
    **Policy:** provide **1:1 ecalls** for these effects (state = 32-byte keys/values; host updates balance/storage trees; metadata & code ops; blob access). [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/instruction-set/)
    

### 6) Cryptographic Instructions

* Fuel includes built-ins for **Keccak-256**, **SHA-256**, signature recovery/verification (secp256k1, secp256r1, ed25519), and **ECOP/EPAR**.  
    **Policy:** expose **deterministic precompiles** as ecalls with the same inputs/outputs/failure modes and assign gas conservatively (initially a reference model that can later be tuned). [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/instruction-set/)
    

### 7) Predicate Execution Semantics

* Fuel has **estimation** and **verification** modes; contract instructions are disallowed in predicate mode; verification ensures gas used matches the input.  
    **Policy:** add a **predicate mode** with a restricted ecall table and exact estimation/verification behavior. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

* * *

## Execution Environment: **libriscv** (Consensus-Safe Configuration)

**Determinism**

* Run **interpreter-only**; **disable JIT/binary translation** across the board to avoid platform variance. Pin a libriscv version in CI. [Medium+1](https://fwsgonzo.medium.com/libriscv-risc-v-binary-translation-part-2-deb3589375ad)
    

**Host ABI integration**

* Implement the **ecall table** below using libriscv’s **syscall handler** mechanism. Validate guest pointers/lengths; return values via `a0–a1`. [libriscv.no](https://libriscv.no/docs/concepts/syscalls/)
    

**Isolation**

* Launch a **fresh VM instance per external/script/predicate/contract call**. Provide the per-call context pointer in a reserved register (see Calling Convention). Zero-init memory; enforce bounds in each ecall. (This mirrors Fuel’s context isolation.) [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

**Instruction stepping & gas**

* Wrap the run loop with a metered **step** that charges for each decoded RISC-V instruction and ecall, mapping to Fuel’s pre-deduct rules. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

* * *

## Target ABI

### Calling Convention

* Baseline: **RV64 SysV** (args in `a0–a7`, returns in `a0–a1`; stack 16-byte aligned, downward; `s0–s11` callee-saved, `t0–t6` caller-saved).
    
* Reserve **`t6` (x31)** as the **environment register** (pointer to per-call context provided by the host).
    
* Custom **ecall** table for host transitions.
    

### Host Interface (ecall IDs; initial)

**Core:**

* `0x100` – Get transaction field (GTF)
    
* `0x101` – Storage read (32-byte slot)
    
* `0x102` – Storage write (32-byte slot)
    
* `0x103` – Log event (`ptr,len`)
    
* `0x104` – Send message/output (SMO-like)
    
* `0x105` – Revert with code
    
* `0x10F` – Gas/accounting (optional control)
    

**Contract & metadata parity (Fuel-like):**

* `0x110` – Call contract (setup, value transfer, context switching)
    
* `0x111` – Transfer to contract / `0x112` – Transfer to output (TR/TRO)
    
* `0x113` – Mint / `0x114` – Burn
    
* `0x115` – Code size / `0x116` – Code copy / `0x117` – Code root / `0x118` – Load code (LDC-like)
    
* `0x119` – Block height / `0x11A` – Block hash / `0x11B` – Timestamp / `0x11C` – Coinbase
    

**Crypto precompiles:**

* `0x120` – Keccak-256 / `0x121` – SHA-256
    
* `0x122` – secp256k1 recover / `0x123` – secp256r1 recover / `0x124` – ed25519 verify
    
* `0x125` – ECOP / `0x126` – EPAR
    

**Blob ops:**

* `0x130` – Blob size (BSIZ) / `0x131` – Blob load (BLDD)
    

**Registers & errors**

* `a0` = call id; `a1–a3` = args/pointers; returns in `a0–a1`; non-zero `a0` indicates error.
    

### Data Layout & Encoding

* Guest endianness: **little-endian** (RISC-V default).
    
* Consensus-visible formats (storage slots, receipts, tx fields, IDs) are fixed **byte strings**; host ecalls adapt endianness as needed to mirror Fuel encodings. Storage keys/values remain **32-byte**. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    

* * *

## IR Intrinsic Refactor

**Today:** `InstOp::FuelVm(FuelVmInstruction)` hard-codes Fuel semantics.

**Plan:**

* Replace with `InstOp::Intrinsic(TargetIntrinsic)`.
    
* Split target-specific parts: `FuelIntrinsic`, `RiscVIntrinsic`.
    
* Categories: environment (GTF/GM), storage, messaging, logging, cryptography, wide arithmetic (>XLEN), traps.
    
* Update passes that pattern-match Fuel intrinsics to the new abstraction.
    
* Parameterize IR helpers over word size/alignment (`XLEN=64` default; 32/128 later).
    

* * *

## Backend Implementation

### `RiscVAsmBuilder` (new: `sway-core/src/asm_generation/riscv`)

* Implements `AsmBuilder`; manages RA, labels, stack frames, data sections.
    
* Emits RV64I/M/B/C with legalization stubs for optional extensions.
    

### Legalization

* Lower `TargetIntrinsic` + high-level IR to RISC-V-friendly pseudo-ops.
    
* Handle pointer arithmetic, immediates, alignment.
    
* Insert libcalls for **256-bit** math, **128-bit** load/store emulation.
    

### Runtime Support Library

* Big-integer helpers, `memcpy/memset`, and other libcall targets (compiled to RISC-V, linked statically or shipped alongside bytecode).
    

* * *

## Intrinsic → RISC-V Mapping (Summary)

* ALU/compare/branches/memory → RV64I/M/B instructions (libcalls for >XLEN).
    
* Calls/returns → `jal/jalr` with RV64 SysV; prologue/epilogue sets **`t6`**.
    
* Storage/Logging/Tx fields/Metadata/Blobs → **ecalls** listed above.
    
* Crypto → **ecalls** (precompiles).
    
* Revert → ecall + trap loop (host terminates).
    
* Memcpy/clear → inline for small sizes, libcall for larger.
    

(Contract/crypto/blob instruction coverage mirrors Fuel’s **BAL/BHEI/BHSH/MINT/BURN/CALL/LOG/LOGD/SCWQ/SRW/SRWQ/SWW/SWWQ/TIME/TR/TRO/SMO**, plus **BSIZ/BLDD** and **Keccak/SHA-256/ECOP/EPAR/curve ops**.) [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/instruction-set/)

* * *

## Tooling & Packaging

* Extend `ProgramABI` with a **RiscV** variant (entry points, ecall table, configurables).
    
* `forc build --target riscv`:
    
    * Use `riscv64-unknown-elf-as` or an embedded assembler crate.
        
    * Emit bytecode + ABI JSON mirroring Fuel/EVM packaging.
        
* Test harness:
    
    * Spin up **libriscv** per test; set registers/stack/context; run to completion; capture receipts and state diffs.
        

* * *

## The Fuel ISA: Why Keep It (for Now)

**Advantages**

* **Spec & compatibility anchor:** a concrete executable spec for existing semantics (receipts, predicates, gas).
    
* **Deterministic baseline:** small, purpose-built ISA that’s easy to audit and model.
    
* **Native contract ops:** storage/logging/calls are single opcodes (good oracle for metering and semantics).
    

**Strategy**

* Keep Fuel as a **reference backend** during migration:
    
    * Use it for **differential testing**: same Sway IR → Fuel vs RISC-V; compare receipts/state/gas.
        
    * Freeze features; no new opcodes.
        
    * Document intentional divergences; add conformance tests.
        

**Off-ramp**

* When RISC-V matches/exceeds Fuel on determinism, gas, and receipts, **deprecate** Fuel to archival status.
    

* * *

## Migration Plan

### Phase 0 — Design Lock

* Finalize ABI choices (ecalls, registers, encoding policy).
    
* Sign off on RVA22U64, interpreter-only libriscv, and per-call VM model.
    

### Phase 1 — Infrastructure

* Add `BuildTarget::RiscV`, CLI plumbed `--target riscv`.
    
* Stub `InstructionSet::RiscV`, skeleton `RiscVAsmBuilder`.
    

### Phase 2 — Intrinsics Refactor

* Introduce `TargetIntrinsic`.
    
* Update Fuel/EVM backends behind a feature flag; ensure no regressions.
    

### Phase 3 — Backend Prototype

* Minimal legalization (integer ops, control flow).
    
* Produce RISC-V assembly for trivial programs.
    

### Phase 4 — Host ABI & Runtime

* Implement ecalls for **GTF/storage/logging/calls/transfers/metadata/blobs/crypto/revert**.
    
* Integrate runtime support library.
    
* **libriscv** wiring: interpreter-only, per-call VM, pointer validation.
    

### Phase 5 — Tooling & Tests

* `forc` packaging for RISC-V artifacts; emulator runner.
    
* **Conformance suite** (Fuel vs RISC-V differential):
    
    * Endianness/encoding fixtures
        
    * Gas parity (pre-deduct)
        
    * Receipts & Merkle root parity
        
    * Predicate estimation/verification
        
    * Contract call isolation & memory ownership
        
* IR/backend unit tests for deterministic codegen.
    

### Phase 6 — Stabilization & Off-ramp

* Harden gas/error semantics; finalize costs.
    
* Document ABI; update tutorials.
    
* Lift feature flags once CI coverage is strong.
    
* Mark Fuel backend **deprecated** once parity is proven.
    

* * *

## Open Questions

* Stay strictly with **RV64 SysV ABI** or add more contract-specific tweaks beyond `t6` and ecalls?
    
* Long-term assembler/toolchain (LLVM `llc` vs riscv-asm crate vs external binutils) and reproducible builds?
    
* How to integrate the RISC-V host runtime with existing node/storage (separate runtime vs integrated)?
    
* Scope/timeline for RV32 (compile-time only initially?) and experimental RV128 gating.
    

* * *

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Large IR refactor surface | Incremental PRs, dual compatibility layers, feature flags |
| ABI design pitfalls | Early consensus with runtime/SDK teams; prototype quickly |
| Emulator variance | Pin libriscv; interpreter-only; cross-platform determinism tests [Medium](https://fwsgonzo.medium.com/libriscv-risc-v-binary-translation-part-2-deb3589375ad) |
| Endianness errors | Enforce host-side encoding policy; add golden fixtures [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/) |
| Gas parity drift | Step-hook metering; differential tests against Fuel semantics [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/) |

* * *

## Appendix A — FuelVM Facts Referenced

* Big-endian integers; 64-bit registers/words; VM init & upward-growing stack; contexts; call-frames & ownership; predicate modes; gas pre-deduction; receipts root update; storage 32-byte keys/values. [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/)
    
* Instruction families: contract ops (BAL/BHEI/BHSH/MINT/BURN/CALL/LOG/LOGD/SCWQ/SRW/SRWQ/SWW/SWWQ/TIME/TR/TRO/SMO), blob ops (BSIZ/BLDD), cryptographic ops (Keccak-256/SHA-256/secp256k1/secp256r1/ed25519/ECOP/EPAR). [Fuel Docs](https://docs.fuel.network/docs/specs/fuel-vm/instruction-set/)
    

## Appendix B — libriscv Notes

* Deterministic baseline is **interpreter mode**; binary translation/JIT exists but must be disabled for consensus. [Medium+1](https://fwsgonzo.medium.com/libriscv-risc-v-binary-translation-part-2-deb3589375ad)
    
* Syscall handler model for host integration (pointer-safe guest memory views; host decides policy). [libriscv.no](https://libriscv.no/docs/concepts/syscalls/)
    

* * *

**Bottom line:** This plan preserves Fuel-level semantics where it matters (encoding, gas, receipts, isolation) while unlocking RISC-V’s tooling ecosystem and long-term portability.