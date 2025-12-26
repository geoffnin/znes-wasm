// Example: Basic PPU test showing initialization and register setup

use znes_wasm::ppu::Ppu;

fn main() {
    println!("SNES PPU Test Example");
    println!("=====================\n");
    
    // Create a new PPU instance
    let mut ppu = Ppu::new();
    println!("✓ PPU initialized");
    println!("  - VRAM: 64KB");
    println!("  - OAM: 544 bytes");
    println!("  - CGRAM: 512 bytes (256 colors)");
    println!("  - Framebuffer: 512×478 RGBA8888\n");
    
    // Setup basic Mode 0 configuration (4 layers, 2bpp each)
    println!("Configuring PPU for Mode 0...");
    ppu.write_register(0x2100, 0x0F); // Enable display, full brightness
    ppu.write_register(0x2105, 0x00); // BG Mode 0
    ppu.write_register(0x212C, 0x01); // Enable BG1 on main screen
    println!("✓ Mode 0 configured\n");
    
    // Setup BG1 tilemap and character data addresses
    println!("Setting up BG1...");
    ppu.write_register(0x2107, 0x00); // BG1 tilemap at VRAM $0000
    ppu.write_register(0x210B, 0x01); // BG1 chr at VRAM $2000
    println!("  - Tilemap address: $0000");
    println!("  - Character data address: $2000\n");
    
    // Write some test palette data (CGRAM)
    println!("Loading test palette...");
    ppu.write_register(0x2121, 0x00); // CGRAM address 0
    
    // Color 0: Black (transparent)
    ppu.write_register(0x2122, 0x00);
    ppu.write_register(0x2122, 0x00);
    
    // Color 1: White
    ppu.write_register(0x2122, 0xFF);
    ppu.write_register(0x2122, 0x7F);
    
    // Color 2: Red
    ppu.write_register(0x2122, 0x1F);
    ppu.write_register(0x2122, 0x00);
    
    // Color 3: Blue
    ppu.write_register(0x2122, 0x00);
    ppu.write_register(0x2122, 0x7C);
    
    println!("✓ Palette loaded (4 colors)\n");
    
    // Write some test VRAM data
    println!("Loading test tile data...");
    ppu.write_register(0x2115, 0x80); // VRAM increment mode: word access, increment on high byte
    ppu.write_register(0x2116, 0x00); // VRAM address $2000 (low)
    ppu.write_register(0x2117, 0x20); // VRAM address $2000 (high)
    
    // Simple checkerboard pattern (2bpp)
    let test_tile = [
        0xAA, 0x55, // Row 0
        0x55, 0xAA, // Row 1
        0xAA, 0x55, // Row 2
        0x55, 0xAA, // Row 3
        0xAA, 0x55, // Row 4
        0x55, 0xAA, // Row 5
        0xAA, 0x55, // Row 6
        0x55, 0xAA, // Row 7
    ];
    
    for &byte in &test_tile {
        ppu.write_register(0x2118, byte); // Low byte
        ppu.write_register(0x2119, 0x00); // High byte
    }
    println!("✓ Test tile loaded\n");
    
    // Simulate a few scanlines
    println!("Simulating PPU rendering...");
    let mut frames_rendered = 0;
    let mut cycles = 0;
    
    while frames_rendered < 1 && cycles < 100000 {
        if ppu.step() {
            frames_rendered += 1;
            println!("✓ Frame {} rendered!", frames_rendered);
            println!("  - Scanlines: 0-262");
            println!("  - Visible scanlines: 0-223");
            println!("  - VBlank: scanlines 225-261");
        }
        cycles += 1;
    }
    
    println!("\nPPU Status:");
    println!("  - Current scanline: {}", ppu.get_scanline());
    println!("  - In VBlank: {}", ppu.in_vblank());
    println!("  - In HBlank: {}", ppu.in_hblank());
    println!("  - Total cycles simulated: {}", cycles);
    
    println!("\n✓ PPU test complete!");
    println!("\nSupported features:");
    println!("  ✓ Mode 0-1 background rendering (4 layers)");
    println!("  ✓ Mode 7 affine transformation");
    println!("  ✓ Sprite rendering (128 sprites, 4bpp)");
    println!("  ✓ Priority system (layer and sprite priorities)");
    println!("  ✓ Tile decoding (2bpp, 4bpp, 8bpp)");
    println!("  ✓ Color palette (256 colors, RGB555)");
    println!("  ✓ Brightness control");
    println!("  ✓ Window effects");
    println!("  ✓ Scanline rendering (0-262)");
    println!("  ✓ Force blank mode");
}
