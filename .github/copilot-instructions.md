# Copilot instructions for znes-wasm

- Keep changes focused and minimal for the requested task.
- This is a Rust + WebAssembly project; preserve `wasm32-unknown-unknown` compatibility.
- Prefer existing module structure in `/src` (`cpu`, `memory`, `ppu`, `apu`, `emulator`, `cartridge`, `chips`) over introducing new abstractions.
- When behavior changes, add or update focused Rust tests under `/tests` or existing `#[cfg(test)]` modules.
- Validate changes with existing project commands:
  - `cargo test`
  - `cargo clippy`
  - `cargo fmt -- --check`
