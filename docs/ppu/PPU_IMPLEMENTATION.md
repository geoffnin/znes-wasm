# SNES PPU (Picture Processing Unit) Implementation

This document describes the implementation of the SNES PPU in the znes-wasm emulator.

## Overview

The PPU is responsible for all graphics rendering on the SNES. It renders scanline-by-scanline during the active display period and provides VBlank/HBlank timing signals.

## Memory Architecture

### VRAM (Video RAM) - 64KB
- Stores tile graphics (character data) and tilemaps
- Accessed via registers $2116-$2119
- Supports multiple increment modes (1, 32, 128 bytes)
- Two-stage access: low byte ($2118) and high byte ($2119)

### OAM (Object Attribute Memory) - 544 bytes
- Main table: 512 bytes (128 sprites × 4 bytes)
- High table: 32 bytes (sprite size and X MSB)
- Accessed via registers $2102-$2104
- Each sprite has: X, Y, tile number, attributes (palette, priority, flip)

### CGRAM (Color RAM) - 512 bytes
- 256 colors × 16-bit RGB555 format
- Colors 0-127: Background palettes
- Colors 128-255: Sprite palettes
- Accessed via registers $2121-$2122
- Color format: `0bbbbbgggggrrrrr` (5 bits per channel)

## Rendering Modes

### Mode 0
- 4 background layers (BG1-BG4)
- All layers use 2bpp (4 colors per tile)
- 8×8 or 16×16 tiles per layer
- Best for games with many layers (e.g., RPGs)

### Mode 1
- BG1: 4bpp (16 colors)
- BG2: 4bpp (16 colors)
- BG3: 2bpp (4 colors)
- Most common mode for action games

### Mode 7
- Single 8bpp background layer (256 colors)
- Affine transformation support (rotation, scaling)
- 128×128 tilemap (1024×1024 pixels)
- Used for 3D effects (Mode 7 racing, world maps)

## Rendering Pipeline

### Scanline Timing
- 341 dots per scanline
- 262 scanlines per frame (NTSC)
- Visible scanlines: 0-223 (224 lines)
- VBlank: scanlines 225-261
- HBlank: after dot 274 each scanline

### Rendering Order (Back to Front)
1. Background layers (by priority)
   - Low priority pixels rendered first
   - High priority pixels rendered second
2. Sprites (by priority 0-3)
3. Apply color math (if enabled)
4. Apply brightness

### Priority System
Each pixel has an associated priority value. Higher priority pixels overwrite lower priority ones:
- BG layers: Priority 0-1 per tile
- Sprites: Priority 0-3
- Final layer order determined by mode and TM register

## Tile Formats

### 2bpp (2 bits per pixel, 4 colors)
- 16 bytes per 8×8 tile
- Used in Mode 0 and Mode 1 (BG3)
- Format: 2 bitplanes interleaved

### 4bpp (4 bits per pixel, 16 colors)
- 32 bytes per 8×8 tile
- Used in Mode 1 (BG1/BG2) and Mode 0-2
- Format: 4 bitplanes, planes 0-1 interleaved, planes 2-3 interleaved

### 8bpp (8 bits per pixel, 256 colors)
- 64 bytes per 8×8 tile
- Used in Mode 3-4 and Mode 7
- Format: 8 bitplanes, sequential pairs

## Key Registers

### Display Control
- **$2100 (INIDISP)**: Display control
  - Bit 7: Force blank (1 = disable display)
  - Bits 0-3: Brightness (0 = black, 15 = full)

### Background Configuration
- **$2105 (BGMODE)**: BG mode and character size
  - Bits 0-2: BG mode (0-7)
  - Bit 3: BG3 priority in Mode 1
  - Bits 4-7: Character size for BG1-4

- **$2107-$210A**: BG tilemap addresses
- **$210B-$210C**: BG character data addresses
- **$210D-$2114**: BG scroll positions (X/Y for each layer)

### VRAM Access
- **$2115**: VRAM address increment mode
- **$2116-$2117**: VRAM address (word address)
- **$2118-$2119**: VRAM data write (low/high byte)
- **$2139-$213A**: VRAM data read (low/high byte)

### Sprite Configuration
- **$2101 (OBSEL)**: Sprite size and base address
  - Bits 0-2: Sprite tile base address
  - Bits 3-4: Name table offset
  - Bits 5-7: Sprite size configuration

### Color/Palette
- **$2121**: CGRAM address
- **$2122**: CGRAM data write (2 writes per color)
- **$213B**: CGRAM data read

### Screen Control
- **$212C (TM)**: Main screen layer enable
  - Bit 0: BG1, Bit 1: BG2, Bit 2: BG3, Bit 3: BG4, Bit 4: OBJ
- **$212D (TS)**: Sub screen layer enable

## Implementation Features

### ✅ Implemented
- [x] Mode 0: 4 layers, 2bpp
- [x] Mode 1: BG1/2 4bpp, BG3 2bpp
- [x] Mode 7: Affine transformation
- [x] Sprite rendering (128 sprites, up to 64×64 pixels)
- [x] Priority system (layers and sprites)
- [x] Tile decoding (2bpp, 4bpp, 8bpp)
- [x] VRAM/OAM/CGRAM access
- [x] Color conversion (RGB555 to RGBA8888)
- [x] Brightness control
- [x] Scanline rendering
- [x] VBlank/HBlank timing
- [x] Horizontal/vertical flip
- [x] Configurable sprite sizes

### 🚧 Partial/Future Enhancements
- [ ] Modes 2-6 (can be added similarly to Mode 0/1)
- [ ] Color math (addition, subtraction)
- [ ] Window masking (registers implemented, rendering not applied)
- [ ] Hi-res modes (512×448)
- [ ] Interlace mode
- [ ] Mosaic effect
- [ ] Offset-per-tile (Mode 2/4/6)
- [ ] Direct color mode (Mode 3/4/7)

## Usage Example

```rust
use znes_wasm::ppu::Ppu;

fn main() {
    let mut ppu = Ppu::new();
    
    // Configure for Mode 0
    ppu.write_register(0x2100, 0x0F); // Full brightness
    ppu.write_register(0x2105, 0x00); // Mode 0
    ppu.write_register(0x212C, 0x01); // Enable BG1
    
    // Set BG1 addresses
    ppu.write_register(0x2107, 0x00); // Tilemap at $0000
    ppu.write_register(0x210B, 0x10); // CHR at $2000
    
    // Load palette
    ppu.write_register(0x2121, 0x00);
    ppu.write_register(0x2122, 0xFF); // Low byte
    ppu.write_register(0x2122, 0x7F); // High byte (white)
    
    // Render frames
    loop {
        if ppu.step() {
            // Frame complete, read framebuffer
            let fb = &ppu.framebuffer;
            // Display or process framebuffer...
        }
    }
}
```

## Performance Considerations

1. **Scanline Rendering**: The PPU renders 224 visible scanlines per frame
2. **Cycle Accuracy**: Each step() call represents 1 master clock cycle
3. **Frame Rate**: NTSC = 60Hz (89,342 cycles/frame)
4. **Framebuffer Size**: 512×478 pixels × 4 bytes = ~976KB

## Reference Documentation

- [SNES Dev Wiki - PPU Registers](https://snes.nesdev.org/wiki/PPU_registers)
- [SNES Dev Wiki - Backgrounds](https://snes.nesdev.org/wiki/Backgrounds)
- [SNES Dev Wiki - Sprites](https://snes.nesdev.org/wiki/Sprites)
- [SNES Dev Wiki - Mode 7](https://snes.nesdev.org/wiki/Mode_7)

## Testing

Run the included example to test PPU functionality:

```bash
cargo run --example ppu_test
```

This will initialize the PPU, configure Mode 0, load test data, and render a frame.
