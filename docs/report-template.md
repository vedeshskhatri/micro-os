# Rust Micro-OS Report

## 1. Abstract

Describe the educational kernel and the mechanisms demonstrated.

## 2. Problem and Objectives

- Build a small Rust kernel laboratory.
- Make scheduling, synchronization, and memory behaviour observable.
- Validate each mechanism with repeatable tests.

## 3. Architecture

Insert the diagram from `docs/architecture.md` and explain the control flow.

## 4. Rust Implementation

Explain the module responsibilities, ownership decisions, and any `unsafe` code.

## 5. Scheduling

Describe Round Robin, timer ticks, task states, task order, and fairness results.

## 6. Synchronization

Explain the spinlock invariant and compare protected versus unprotected access.

## 7. Memory Management

Explain bump allocation, alignment, capacity, failure behaviour, and limitations.

## 8. Testing and Evidence

Include `cargo test` output, the demo trace, and trace-analysis results.

## 9. Limitations and Future Work

The next implementation milestone is a QEMU boot target with serial output, hardware timer interrupts, and CPU context switching.

## 10. AI Usage Disclosure

Record which design, syntax, debugging, or documentation tasks used AI assistance. Explain how the team verified and understood the submitted code.
