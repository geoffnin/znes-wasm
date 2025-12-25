# ZNES-WASM

A SNES emulator compiled to WebAssembly.

## Building for WASM

### Build

Build for web:
```bash
wasm-pack build --target web
```

Build for Node.js:
```bash
wasm-pack build --target nodejs
```

Build for bundlers (webpack, etc.):
```bash
wasm-pack build --target bundler
```

### Development

Run tests:
```bash
cargo test
```

Run WASM tests:
```bash
wasm-pack test --headless --firefox
```

### Project Structure

- `src/lib.rs` - Library crate root with WASM bindings
- `Cargo.toml` - Project configuration with WASM optimizations
- `.cargo/config.toml` - Cargo configuration for WASM target

### Usage in JavaScript

After building with wasm-pack, you can use it in JavaScript:

```javascript
import init, { greet, add } from './pkg/znes_wasm.js';

await init();
console.log(greet('World'));
console.log(add(2, 3));
```
