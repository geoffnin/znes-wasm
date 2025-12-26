# SNES PPU Register Reference

Quick reference for all implemented PPU registers in znes-wasm.

## Display Control

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2100 | INIDISP | Display Control & Brightness | W |
| | Bit 7 | Force Blank (1=off) | |
| | Bits 0-3 | Brightness (0-15) | |

## Background Configuration

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2105 | BGMODE | BG Mode & Character Size | W |
| | Bits 0-2 | BG Mode (0-7) | |
| | Bit 3 | BG3 Priority in Mode 1 | |
| | Bits 4-7 | BG1-4 Tile Size (8×8 or 16×16) | |
| $2106 | MOSAIC | Mosaic Size & Enable | W |
| $2107 | BG1SC | BG1 Tilemap Address & Size | W |
| $2108 | BG2SC | BG2 Tilemap Address & Size | W |
| $2109 | BG3SC | BG3 Tilemap Address & Size | W |
| $210A | BG4SC | BG4 Tilemap Address & Size | W |
| $210B | BG12NBA | BG1 & BG2 Character Data Address | W |
| $210C | BG34NBA | BG3 & BG4 Character Data Address | W |

## Background Scroll

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $210D | BG1HOFS | BG1 Horizontal Scroll (write twice) | W×2 |
| $210E | BG1VOFS | BG1 Vertical Scroll (write twice) | W×2 |
| $210F | BG2HOFS | BG2 Horizontal Scroll | W×2 |
| $2110 | BG2VOFS | BG2 Vertical Scroll | W×2 |
| $2111 | BG3HOFS | BG3 Horizontal Scroll | W×2 |
| $2112 | BG3VOFS | BG3 Vertical Scroll | W×2 |
| $2113 | BG4HOFS | BG4 Horizontal Scroll | W×2 |
| $2114 | BG4VOFS | BG4 Vertical Scroll | W×2 |

## VRAM Access

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2115 | VMAIN | VRAM Address Increment Mode | W |
| | Bits 0-1 | Increment (0=1, 1=32, 2/3=128) | |
| | Bit 2 | Mapping (0=low, 1=high) | |
| $2116 | VMADDL | VRAM Address Low | W |
| $2117 | VMADDH | VRAM Address High | W |
| $2118 | VMDATAL | VRAM Data Write Low | W |
| $2119 | VMDATAH | VRAM Data Write High | W |
| $2139 | VMDATALREAD | VRAM Data Read Low | R |
| $213A | VMDATAHREAD | VRAM Data Read High | R |

## Mode 7

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $211A | M7SEL | Mode 7 Settings | W |
| $211B | M7A | Mode 7 Matrix A (write twice) | W×2 |
| $211C | M7B | Mode 7 Matrix B (write twice) | W×2 |
| $211D | M7C | Mode 7 Matrix C (write twice) | W×2 |
| $211E | M7D | Mode 7 Matrix D (write twice) | W×2 |
| $211F | M7X | Mode 7 Center X (write twice) | W×2 |
| $2120 | M7Y | Mode 7 Center Y (write twice) | W×2 |

## OAM (Sprite) Access

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2101 | OBSEL | Sprite Size & Base Address | W |
| | Bits 0-2 | Base Address (×$2000) | |
| | Bits 3-4 | Name Table Offset | |
| | Bits 5-7 | Sprite Size | |
| $2102 | OAMADDL | OAM Address Low | W |
| $2103 | OAMADDH | OAM Address High | W |
| $2104 | OAMDATA | OAM Data Write | W |
| $2138 | OAMDATAREAD | OAM Data Read | R |

## CGRAM (Palette) Access

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2121 | CGADD | CGRAM Address | W |
| $2122 | CGDATA | CGRAM Data Write (write twice) | W×2 |
| $213B | CGDATAREAD | CGRAM Data Read | R |

## Window Masking

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2123 | W12SEL | Window Mask Settings for BG1/BG2 | W |
| $2124 | W34SEL | Window Mask Settings for BG3/BG4 | W |
| $2125 | WOBJSEL | Window Mask Settings for OBJ/Color | W |
| $2126 | WH0 | Window 1 Left Position | W |
| $2127 | WH1 | Window 1 Right Position | W |
| $2128 | WH2 | Window 2 Left Position | W |
| $2129 | WH3 | Window 2 Right Position | W |
| $212A | WBGLOG | Window Mask Logic for BG1-4 | W |
| $212B | WOBJLOG | Window Mask Logic for OBJ & Color | W |

## Screen Designation

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $212C | TM | Main Screen Designation | W |
| | Bit 0 | BG1 Enable | |
| | Bit 1 | BG2 Enable | |
| | Bit 2 | BG3 Enable | |
| | Bit 3 | BG4 Enable | |
| | Bit 4 | OBJ Enable | |
| $212D | TS | Sub Screen Designation | W |
| $212E | TMW | Window Mask for Main Screen | W |
| $212F | TSW | Window Mask for Sub Screen | W |

## Color Math

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2130 | CGWSEL | Color Math Control | W |
| $2131 | CGADSUB | Color Math Designation | W |
| $2132 | COLDATA | Fixed Color Data | W |
| | Bit 5 | Red Component Enable | |
| | Bit 6 | Green Component Enable | |
| | Bit 7 | Blue Component Enable | |
| | Bits 0-4 | Color Value (0-31) | |

## PPU Status

| Address | Name | Description | Access |
|---------|------|-------------|--------|
| $2134 | MPYL | Multiplication Result Low | R |
| $2135 | MPYM | Multiplication Result Middle | R |
| $2136 | MPYH | Multiplication Result High | R |
| $2137 | SLHV | Software Latch for H/V Counters | R |
| $213C | OPHCT | Horizontal Counter Latch | R |
| $213D | OPVCT | Vertical Counter Latch | R |
| $213E | STAT77 | PPU1 Status & Version | R |
| | Bit 7 | Time Over Flag | |
| | Bit 6 | Range Over Flag | |
| | Bits 0-4 | Version (5C77) | |
| $213F | STAT78 | PPU2 Status & Version | R |
| | Bit 7 | Interlace Field / VBlank | |
| | Bit 6 | External Latch / HBlank | |
| | Bits 0-4 | Version (5C78) | |

## Sprite Size Configurations (OBSEL bits 5-7)

| Value | Small Size | Large Size |
|-------|------------|------------|
| 0 | 8×8 | 16×16 |
| 1 | 8×8 | 32×32 |
| 2 | 8×8 | 64×64 |
| 3 | 16×16 | 32×32 |
| 4 | 16×16 | 64×64 |
| 5 | 32×32 | 64×64 |
| 6 | 16×32 | 32×64 |
| 7 | 16×32 | 32×32 |

## BG Modes

| Mode | BG1 | BG2 | BG3 | BG4 | Colors | Notes |
|------|-----|-----|-----|-----|--------|-------|
| 0 | 2bpp | 2bpp | 2bpp | 2bpp | 4/4/4/4 | 4 layers |
| 1 | 4bpp | 4bpp | 2bpp | - | 16/16/4 | Most common |
| 2 | 4bpp | 4bpp | - | - | 16/16 | Offset-per-tile |
| 3 | 8bpp | 4bpp | - | - | 256/16 | Direct color |
| 4 | 8bpp | 2bpp | - | - | 256/4 | Offset-per-tile |
| 5 | 4bpp | 2bpp | - | - | 16/4 | Hi-res (512px) |
| 6 | 4bpp | - | - | - | 16 | Hi-res, offset-per-tile |
| 7 | 8bpp | - | - | - | 256 | Rotation/scaling |

## Color Format (RGB555)

SNES colors are 15-bit RGB555 format:
```
Bit: 15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
     0  b  b  b  b  b  g  g  g  g  g  r  r  r  r  r
```

Each channel is 5 bits (0-31), giving 32,768 possible colors.

## Tilemap Entry Format (2 bytes)

```
High Byte (Attributes):
  Bit 7: V-Flip
  Bit 6: H-Flip
  Bit 5: Priority
  Bits 4-2: Palette (0-7)
  Bits 1-0: Character Number High (bits 9-8)

Low Byte:
  Bits 7-0: Character Number Low (bits 7-0)
```

## OAM Entry Format (4 bytes + high table)

Main Table (4 bytes per sprite):
```
Byte 0: X Position (low 8 bits)
Byte 1: Y Position
Byte 2: Tile Number
Byte 3: Attributes
  Bit 7: V-Flip
  Bit 6: H-Flip
  Bits 5-4: Priority (0-3)
  Bits 3-1: Palette (0-7) + 8
  Bit 0: Name Table Select
```

High Table (2 bits per sprite):
```
Bit 1: Size (0=small, 1=large)
Bit 0: X Position MSB (bit 8)
```

## Notes

- Write-twice registers use a latch system (write low byte, then high byte)
- VRAM word address is doubled for byte addressing
- OAM has 128 sprites with 4 bytes main + high table
- CGRAM addresses are byte-aligned but data is 2 bytes per color
- Mode 7 uses 13.3 fixed-point for transformation matrix
