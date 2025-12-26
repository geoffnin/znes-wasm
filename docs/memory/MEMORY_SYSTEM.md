# SNES Memory System and ROM Cartridge Loading

This document describes the implementation of the SNES memory system and ROM cartridge loading in the znes-wasm emulator.

## Overview

The SNES (Super Nintendo Entertainment System) uses a sophisticated memory architecture with:
- 24-bit addressing (16MB address space)
- 256 banks of 64KB each
- Multiple mapping modes (LoROM, HiROM, ExHiROM)
- Bank switching and memory mirroring
- Various memory types (WRAM, SRAM, ROM)

## Architecture

### Memory Module (`src/memory.rs`)

The `Memory` struct manages the entire SNES memory system with:

#### Components

1. **WRAM (Work RAM)**: 128KB of general-purpose RAM
   - Banks $7E-$7F contain the full 128KB
   - Lower 8KB mirrored in banks $00-$3F and $80-$BF at $0000-$1FFF

2. **SRAM (Save RAM)**: Variable size (0-32KB typically)
   - Battery-backed RAM for save data
   - Location depends on mapping mode

3. **ROM**: Cartridge ROM data
   - Size varies by game
   - Mapped differently based on cartridge type

#### Mapping Modes

##### LoROM (Low ROM)
- ROM mapped to upper half ($8000-$FFFF) of banks $00-$7D and $80-$FD
- Lower half contains WRAM mirrors and I/O
- SRAM in banks $70-$7D at $8000-$FFFF
- Common for smaller games

##### HiROM (High ROM)
- ROM mapped more linearly starting at bank $C0
- Banks $00-$3F and $80-$BF have ROM in upper half
- SRAM in banks $00-$3F at $6000-$7FFF
- More efficient for larger games

##### ExHiROM (Extended High ROM)
- Extended version of HiROM for very large ROMs (up to 8MB)
- Special extended addressing for banks $40-$7D

#### Fast Address Translation

The Memory system uses lookup tables for O(1) address translation:
- `read_map[2048]`: Maps 8KB pages to readable memory regions
- `write_map[2048]`: Maps 8KB pages to writable memory regions

Each entry contains:
- Region type (WRAM, SRAM, ROM, or None)
- Offset into the actual memory array

This eliminates the need for complex address calculations on every memory access.

### Cartridge Module (`src/cartridge.rs`)

The `Cartridge` struct handles ROM loading and header parsing.

#### ROM Format Support

1. **.sfc files**: Raw ROM data without header
2. **.smc files**: ROM with 512-byte copier header (automatically detected and stripped)

#### Header Detection

The cartridge loader:
1. Checks for .smc header (file size % 1024 == 512)
2. Searches for valid headers at:
   - $7FC0 (LoROM location)
   - $FFC0 (HiROM location)
3. Scores each location based on validity checks
4. Selects the most likely valid header

#### Header Information

Extracted from ROM:
- **Title**: 21-character ASCII string
- **Mapping Mode**: LoROM, HiROM, or ExHiROM
- **Region**: Japan, North America, Europe, etc.
- **Cartridge Type**: ROM only, ROM+RAM, ROM+RAM+Battery, etc.
- **ROM Size**: Calculated from header byte (2^n KB)
- **SRAM Size**: Calculated from header byte (2^n KB)

## Usage Examples

### Loading a ROM

```rust
use znes_wasm::cartridge::Cartridge;
use znes_wasm::memory::Memory;

// Load ROM from file data
let rom_data = std::fs::read("game.sfc")?;
let cartridge = Cartridge::from_rom(rom_data)?;

// Create memory system
let mut memory = Memory::new(&cartridge);
```

### Reading/Writing Memory

```rust
// Read byte from WRAM
let value = memory.read(0x7E0000);

// Write byte to WRAM
memory.write(0x7E0000, 0x42);

// Read 16-bit word (little-endian)
let word = memory.read_word(0x7E0000);

// Write 16-bit word
memory.write_word(0x7E0000, 0x1234);
```

### Reading ROM Data

```rust
// LoROM: ROM starts at $8000 in bank $00
let rom_byte = memory.read(0x008000);

// HiROM: ROM starts at $8000 in bank $00 (but mapped differently)
let rom_byte = memory.read(0x008000);
```

### Working with SRAM

```rust
// Save SRAM to file
let sram_data = memory.sram();
std::fs::write("game.srm", sram_data)?;

// Load SRAM from file
let sram_data = std::fs::read("game.srm")?;
memory.load_sram(&sram_data);
```

### Cartridge Information

```rust
let cartridge = Cartridge::from_rom(rom_data)?;

println!("Title: {}", cartridge.title());
println!("Region: {:?}", cartridge.region());
println!("Mapping: {:?}", cartridge.mapping_mode());
println!("ROM Size: {} bytes", cartridge.rom_size());
println!("SRAM Size: {} bytes", cartridge.sram_size());
```

## Memory Map Reference

### LoROM Memory Map

| Banks    | Address Range | Description                    |
|----------|---------------|--------------------------------|
| $00-$3F  | $0000-$1FFF   | WRAM (first 8KB, mirrored)    |
| $00-$3F  | $2000-$5FFF   | I/O Registers                 |
| $00-$3F  | $8000-$FFFF   | ROM (32KB per bank)           |
| $40-$6F  | $8000-$FFFF   | Extended ROM                  |
| $70-$7D  | $8000-$FFFF   | SRAM                          |
| $7E-$7F  | $0000-$FFFF   | WRAM (full 128KB)             |
| $80-$BF  | $0000-$1FFF   | WRAM mirror                   |
| $80-$BF  | $8000-$FFFF   | ROM mirror                    |
| $C0-$EF  | $8000-$FFFF   | Extended ROM mirror           |
| $F0-$FD  | $8000-$FFFF   | SRAM mirror                   |

### HiROM Memory Map

| Banks    | Address Range | Description                    |
|----------|---------------|--------------------------------|
| $00-$3F  | $0000-$1FFF   | WRAM (first 8KB, mirrored)    |
| $00-$3F  | $2000-$5FFF   | I/O Registers                 |
| $00-$3F  | $6000-$7FFF   | SRAM                          |
| $00-$3F  | $8000-$FFFF   | ROM (32KB per bank)           |
| $40-$7D  | $0000-$FFFF   | ROM (64KB per bank)           |
| $7E-$7F  | $0000-$FFFF   | WRAM (full 128KB)             |
| $80-$BF  | $0000-$1FFF   | WRAM mirror                   |
| $80-$BF  | $8000-$FFFF   | ROM mirror                    |
| $C0-$FF  | $0000-$FFFF   | ROM (64KB per bank)           |

## Testing

The implementation includes comprehensive unit tests covering:

### Cartridge Tests
- LoROM header detection
- HiROM header detection
- .smc header removal
- Cartridge type detection
- Region detection

### Memory Tests
- WRAM access and mirroring
- ROM access in different mapping modes
- SRAM access and persistence
- 16-bit word access
- Bank switching

Run tests with:
```bash
cargo test --lib --target x86_64-unknown-linux-gnu
```

## Performance Considerations

1. **Lookup Tables**: O(1) address translation using pre-computed tables
2. **Boxed WRAM**: 128KB array is heap-allocated to avoid stack overflow
3. **Memory Mirroring**: Handled in lookup table initialization, not at runtime
4. **Bounds Checking**: Minimal overhead with modulo operations for wrap-around

## References

- [SNES Development Wiki - Memory Map](https://snes.nesdev.org/wiki/Memory_map)
- [SNES Development Wiki - ROM header](https://snes.nesdev.org/wiki/ROM_header)
- [Fullsnes Documentation](https://problemkaputt.de/fullsnes.htm)

## Future Enhancements

Potential improvements for future versions:

1. **I/O Register Implementation**: Currently unmapped
2. **SA-1 and Other Coprocessors**: Special chip support
3. **ExLoROM Support**: Another extended mapping mode
4. **Checksum Validation**: Verify ROM integrity
5. **Open Bus Behavior**: More accurate unmapped region handling
6. **DMA Channels**: Direct memory access support
