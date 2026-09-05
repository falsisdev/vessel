# ADR 0002: Protocol Buffers and gRPC as Inter-Component Contract

## Status
Accepted

## Context
Plugins will be authored by community developers in diverse languages (Go, Rust, Python, Kotlin, C#). Furthermore, future iterations may adopt WebAssembly (Wasm) or shared libraries. We need a strongly typed, backward-compatible, language-agnostic contract.

## Decision
1. **Source of Truth**: Define all inter-process boundaries using Protocol Buffers (`proto/plugin/v1/plugin.proto`).
2. **Transport**: Use gRPC for remote procedure calls, providing streaming, cancellation, timeouts, and metadata passing out of the box.
3. **Toolchain**: Use `buf` (`buf lint`, `buf generate`) for standardized linting, breaking change detection, and code generation.
4. **Versioning**: Enforce explicit package versioning (e.g., `plugin.v1`). Breaking changes will introduce `plugin.v2` without mutating active `v1` schemas.

## Consequences
- Clean language separation with zero hand-written duplicate models across ecosystems.
- Explicit schema definitions prevent subtle runtime serialization bugs.
- Requires protoc/buf toolchain during code generation, but no runtime compiler dependency.
