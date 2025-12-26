// SNES PPU (Picture Processing Unit) Implementation
// Reference: https://snes.nesdev.org/wiki/PPU_registers

/// SNES PPU - handles all graphics rendering
pub struct Ppu {
    // Memory regions
    vram: [u8; 0x10000],        // 64KB Video RAM
    oam: [u8; 544],              // 544 bytes Object Attribute Memory
    cgram: [u16; 256],           // 512 bytes Color RAM (256 colors, 16-bit each)
    
    // Output framebuffer (RGBA8888, sized to handle overscan)
    pub framebuffer: Vec<u32>,
    
    // Scanline tracking
    scanline: u16,               // Current scanline (0-262)
    dot: u16,                    // Current dot/cycle in scanline
    
    // VRAM access
    vram_address: u16,
    vram_increment: u16,
    vram_mapping: VramMapping,
    vram_read_buffer: u16,
    
    // OAM access
    oam_address: u16,
    oam_high_byte: bool,
    oam_latch: u8,
    
    // CGRAM access
    cgram_address: u8,
    cgram_latch: u8,
    cgram_high_byte: bool,
    
    // PPU Control Registers
    inidisp: u8,                 // $2100 - Display control (brightness, force blank)
    obsel: u8,                   // $2101 - Object size and character data address
    oamadd: u16,                 // $2102-2103 - OAM address
    bgmode: u8,                  // $2105 - BG mode and character size
    mosaic: u8,                  // $2106 - Mosaic size and enable
    
    // Background control
    bg1_tilemap_addr: u16,       // BG1 tilemap address
    bg2_tilemap_addr: u16,       // BG2 tilemap address
    bg3_tilemap_addr: u16,       // BG3 tilemap address
    bg4_tilemap_addr: u16,       // BG4 tilemap address
    bg1_chr_addr: u16,           // BG1 character data address
    bg2_chr_addr: u16,           // BG2 character data address
    bg3_chr_addr: u16,           // BG3 character data address
    bg4_chr_addr: u16,           // BG4 character data address
    
    // Background scroll positions
    bg1_hscroll: u16,
    bg1_vscroll: u16,
    bg2_hscroll: u16,
    bg2_vscroll: u16,
    bg3_hscroll: u16,
    bg3_vscroll: u16,
    bg4_hscroll: u16,
    bg4_vscroll: u16,
    
    // Scroll latches (PPU uses prev/current write pattern)
    bg_scroll_latch: u8,
    
    // Window configuration
    window1_left: u8,
    window1_right: u8,
    window2_left: u8,
    window2_right: u8,
    window_mask_settings: [u8; 6],  // For BG1-4, OBJ, and Color
    window_mask_logic: [u8; 6],
    
    // Main/Sub screen designation
    tm: u8,                      // $212C - Main screen designation
    ts: u8,                      // $212D - Sub screen designation
    tmw: u8,                     // $212E - Window mask for main screen
    tsw: u8,                     // $212F - Window mask for sub screen
    
    // Color math
    cgwsel: u8,                  // $2130 - Color math control
    cgadsub: u8,                 // $2131 - Color math designation
    coldata: u8,                 // $2132 - Fixed color data
    fixed_color: [u8; 3],        // RGB components
    
    // Mode 7 registers
    m7sel: u8,
    m7a: i16,
    m7b: i16,
    m7c: i16,
    m7d: i16,
    m7x: i16,
    m7y: i16,
    m7_latch: u8,
    
    // Status
    stat77: u8,                  // $2137 - Software latch for H/V counters
    stat78: u8,                  // $2138 - OAM data read
    ophct: u16,                  // Horizontal counter
    opvct: u16,                  // Vertical counter
    
    // Rendering state
    vblank: bool,
    hblank: bool,
    frame_complete: bool,
}

#[derive(Debug, Clone, Copy)]
enum VramMapping {
    Increment0,                  // Increment after low byte access
    Increment1,                  // Increment after high byte access
}

#[derive(Debug, Clone, Copy)]
struct BgLayer {
    enabled: bool,
    tilemap_addr: u16,
    chr_addr: u16,
    hscroll: u16,
    vscroll: u16,
    tile_size: TileSize,         // 8x8 or 16x16
    tilemap_width: u16,          // In tiles
    tilemap_height: u16,         // In tiles
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TileSize {
    Size8x8,
    Size16x16,
}

#[derive(Debug, Clone, Copy)]
struct Sprite {
    x: i16,
    y: u8,
    tile: u16,
    palette: u8,
    priority: u8,
    flip_h: bool,
    flip_v: bool,
    size: SpriteSize,
}

#[derive(Debug, Clone, Copy)]
enum SpriteSize {
    Small,
    Large,
}

impl Ppu {
    pub fn new() -> Self {
        let mut ppu = Self {
            vram: [0; 0x10000],
            oam: [0; 544],
            cgram: [0; 256],
            framebuffer: vec![0; 512 * 478],
            
            scanline: 0,
            dot: 0,
            
            vram_address: 0,
            vram_increment: 1,
            vram_mapping: VramMapping::Increment0,
            vram_read_buffer: 0,
            
            oam_address: 0,
            oam_high_byte: false,
            oam_latch: 0,
            
            cgram_address: 0,
            cgram_latch: 0,
            cgram_high_byte: false,
            
            inidisp: 0x80,           // Force blank on startup
            obsel: 0,
            oamadd: 0,
            bgmode: 0,
            mosaic: 0,
            
            bg1_tilemap_addr: 0,
            bg2_tilemap_addr: 0,
            bg3_tilemap_addr: 0,
            bg4_tilemap_addr: 0,
            bg1_chr_addr: 0,
            bg2_chr_addr: 0,
            bg3_chr_addr: 0,
            bg4_chr_addr: 0,
            
            bg1_hscroll: 0,
            bg1_vscroll: 0,
            bg2_hscroll: 0,
            bg2_vscroll: 0,
            bg3_hscroll: 0,
            bg3_vscroll: 0,
            bg4_hscroll: 0,
            bg4_vscroll: 0,
            
            bg_scroll_latch: 0,
            
            window1_left: 0,
            window1_right: 0,
            window2_left: 0,
            window2_right: 0,
            window_mask_settings: [0; 6],
            window_mask_logic: [0; 6],
            
            tm: 0,
            ts: 0,
            tmw: 0,
            tsw: 0,
            
            cgwsel: 0,
            cgadsub: 0,
            coldata: 0,
            fixed_color: [0, 0, 0],
            
            m7sel: 0,
            m7a: 0,
            m7b: 0,
            m7c: 0,
            m7d: 0,
            m7x: 0,
            m7y: 0,
            m7_latch: 0,
            
            stat77: 0,
            stat78: 0,
            ophct: 0,
            opvct: 0,
            
            vblank: true,
            hblank: false,
            frame_complete: false,
        };
        
        // Initialize with black color palette
        for i in 0..256 {
            ppu.cgram[i] = 0;
        }
        
        ppu
    }
    
    /// Run PPU for one master clock cycle
    pub fn step(&mut self) -> bool {
        self.frame_complete = false;
        
        // SNES timing: 341 dots per scanline, 262 scanlines per frame (NTSC)
        self.dot += 1;
        
        if self.dot >= 341 {
            self.dot = 0;
            self.scanline += 1;
            
            // Render scanline if visible
            if self.scanline < 224 && !self.is_forced_blank() {
                self.render_scanline();
            }
            
            if self.scanline == 225 {
                // Start of VBlank
                self.vblank = true;
            }
            
            if self.scanline >= 262 {
                self.scanline = 0;
                self.vblank = false;
                self.frame_complete = true;
            }
        }
        
        // HBlank occurs after visible portion (dot 274+)
        self.hblank = self.dot >= 274;
        
        self.frame_complete
    }
    
    /// Check if display is force blanked
    fn is_forced_blank(&self) -> bool {
        (self.inidisp & 0x80) != 0
    }
    
    /// Get current brightness level (0-15)
    fn get_brightness(&self) -> u8 {
        self.inidisp & 0x0F
    }
    
    /// Render the current scanline
    fn render_scanline(&mut self) {
        let y = self.scanline as usize;
        if y >= 224 {
            return;
        }
        
        // Create scanline buffer
        let mut scanline_buffer = [0u32; 512];
        let mut priority_buffer = [0u8; 512];
        
        let mode = self.bgmode & 0x07;
        
        match mode {
            0 => self.render_mode0(&mut scanline_buffer, &mut priority_buffer),
            1 => self.render_mode1(&mut scanline_buffer, &mut priority_buffer),
            7 => self.render_mode7(&mut scanline_buffer, &mut priority_buffer),
            _ => {
                // Other modes not yet implemented - render black
                for x in 0..256 {
                    scanline_buffer[x] = self.rgb555_to_rgba8888(0);
                }
            }
        }
        
        // Render sprites on top
        self.render_sprites(&mut scanline_buffer, &mut priority_buffer);
        
        // Apply brightness
        let brightness = self.get_brightness();
        if brightness < 15 {
            for pixel in scanline_buffer.iter_mut() {
                *pixel = self.apply_brightness(*pixel, brightness);
            }
        }
        
        // Copy to framebuffer
        let fb_offset = y * 512;
        for x in 0..256 {
            self.framebuffer[fb_offset + x] = scanline_buffer[x];
        }
    }
    
    /// Render Mode 0 - 4 layers, 2bpp each
    fn render_mode0(&mut self, scanline: &mut [u32; 512], priority: &mut [u8; 512]) {
        // Mode 0: BG1-4 all 2bpp, 8x8 or 16x16 tiles
        let bg3_priority = (self.bgmode & 0x08) != 0;
        
        // Render in priority order (back to front)
        // Priority: BG4 low, BG3 low, BG2 low, BG1 low, BG4 high, BG3 high, BG2 high, BG1 high
        
        if self.tm & 0x08 != 0 {  // BG4 enabled on main screen
            self.render_bg_layer(3, 0, scanline, priority, 2); // Priority 0
        }
        
        if self.tm & 0x04 != 0 {  // BG3 enabled
            self.render_bg_layer(2, 0, scanline, priority, if bg3_priority { 8 } else { 4 });
        }
        
        if self.tm & 0x02 != 0 {  // BG2 enabled
            self.render_bg_layer(1, 0, scanline, priority, 6);
        }
        
        if self.tm & 0x01 != 0 {  // BG1 enabled
            self.render_bg_layer(0, 0, scanline, priority, 10);
        }
        
        // High priority versions
        if self.tm & 0x08 != 0 {
            self.render_bg_layer(3, 1, scanline, priority, 3);
        }
        
        if self.tm & 0x04 != 0 {
            self.render_bg_layer(2, 1, scanline, priority, if bg3_priority { 9 } else { 5 });
        }
        
        if self.tm & 0x02 != 0 {
            self.render_bg_layer(1, 1, scanline, priority, 7);
        }
        
        if self.tm & 0x01 != 0 {
            self.render_bg_layer(0, 1, scanline, priority, 11);
        }
    }
    
    /// Render Mode 1 - BG1/BG2 4bpp, BG3 2bpp
    fn render_mode1(&mut self, scanline: &mut [u32; 512], priority: &mut [u8; 512]) {
        // Mode 1: BG1 and BG2 are 4bpp, BG3 is 2bpp (background)
        let bg3_priority = (self.bgmode & 0x08) != 0;
        
        if self.tm & 0x04 != 0 {  // BG3 enabled (lowest priority)
            self.render_bg_layer(2, 0, scanline, priority, if bg3_priority { 8 } else { 2 });
        }
        
        if self.tm & 0x02 != 0 {  // BG2 low priority
            self.render_bg_layer(1, 0, scanline, priority, 4);
        }
        
        if self.tm & 0x01 != 0 {  // BG1 low priority
            self.render_bg_layer(0, 0, scanline, priority, 6);
        }
        
        if self.tm & 0x04 != 0 {  // BG3 high priority
            self.render_bg_layer(2, 1, scanline, priority, if bg3_priority { 9 } else { 3 });
        }
        
        if self.tm & 0x02 != 0 {  // BG2 high priority
            self.render_bg_layer(1, 1, scanline, priority, 5);
        }
        
        if self.tm & 0x01 != 0 {  // BG1 high priority
            self.render_bg_layer(0, 1, scanline, priority, 7);
        }
    }
    
    /// Render Mode 7 - Affine transformation mode
    fn render_mode7(&mut self, scanline: &mut [u32; 512], priority: &mut [u8; 512]) {
        // Mode 7 is a special affine transformation mode
        // For now, just render a simple version
        
        let y = self.scanline as i32;
        let repeat = (self.m7sel & 0xC0) >> 6;
        
        for x in 0..256 {
            // Apply affine transformation
            let screen_x = x as i32 - 128;
            let screen_y = y - 112;
            
            // Transform to tilemap space
            let tile_x = ((self.m7a as i32 * screen_x + self.m7b as i32 * screen_y) >> 8) + (self.m7x as i32);
            let tile_y = ((self.m7c as i32 * screen_x + self.m7d as i32 * screen_y) >> 8) + (self.m7y as i32);
            
            // Check bounds based on repeat mode
            let (tx, ty) = match repeat {
                0 => {
                    // Repeat in both directions
                    ((tile_x & 0x3FF) as u16, (tile_y & 0x3FF) as u16)
                },
                1 | 2 => {
                    // Out of bounds = transparent
                    if tile_x < 0 || tile_x >= 1024 || tile_y < 0 || tile_y >= 1024 {
                        continue;
                    }
                    (tile_x as u16, tile_y as u16)
                },
                _ => continue,
            };
            
            // Get tile and pixel from Mode 7 tilemap
            let tile_num = self.read_vram(((ty >> 3) * 128 + (tx >> 3)) as u16) as u16;
            let pixel_x = (tx & 7) as usize;
            let pixel_y = (ty & 7) as usize;
            
            let tile_addr = tile_num * 64 + (pixel_y * 8 + pixel_x) as u16;
            let color_index = self.read_vram(tile_addr);
            
            if color_index != 0 {
                let color = self.cgram[color_index as usize];
                scanline[x] = self.rgb555_to_rgba8888(color);
                priority[x] = 8;
            }
        }
    }
    
    /// Render a background layer
    fn render_bg_layer(&mut self, bg_num: usize, pri: u8, scanline: &mut [u32; 512], priority: &mut [u8; 512], layer_priority: u8) {
        let y = self.scanline;
        
        let (tilemap_addr, chr_addr, hscroll, vscroll) = match bg_num {
            0 => (self.bg1_tilemap_addr, self.bg1_chr_addr, self.bg1_hscroll, self.bg1_vscroll),
            1 => (self.bg2_tilemap_addr, self.bg2_chr_addr, self.bg2_hscroll, self.bg2_vscroll),
            2 => (self.bg3_tilemap_addr, self.bg3_chr_addr, self.bg3_hscroll, self.bg3_vscroll),
            3 => (self.bg4_tilemap_addr, self.bg4_chr_addr, self.bg4_hscroll, self.bg4_vscroll),
            _ => return,
        };
        
        let mode = self.bgmode & 0x07;
        let bpp = self.get_bg_bpp(bg_num, mode);
        let _tile_size = if self.is_bg_16x16(bg_num) { TileSize::Size16x16 } else { TileSize::Size8x8 };
        
        // Adjust Y position with scroll
        let tile_y = ((y as u16 + vscroll) & 0x1FF) as usize;
        let fine_y = tile_y & 7;
        let coarse_y = tile_y >> 3;
        
        for x in 0..256 {
            // Adjust X position with scroll
            let tile_x = ((x as u16 + hscroll) & 0x1FF) as usize;
            let fine_x = tile_x & 7;
            let coarse_x = tile_x >> 3;
            
            // Get tilemap entry (32x32 tiles = 1024 entries, 2 bytes each)
            let tilemap_offset = tilemap_addr + ((coarse_y * 32 + coarse_x) * 2) as u16;
            let tile_low = self.read_vram(tilemap_offset);
            let tile_high = self.read_vram(tilemap_offset + 1);
            
            let tile_num = (tile_low as u16) | (((tile_high & 0x03) as u16) << 8);
            let palette = (tile_high >> 2) & 0x07;
            let tile_pri = (tile_high >> 5) & 0x01;
            let flip_h = (tile_high & 0x40) != 0;
            let flip_v = (tile_high & 0x80) != 0;
            
            // Skip if wrong priority
            if tile_pri != pri {
                continue;
            }
            
            // Get pixel from tile
            let pixel_x = if flip_h { 7 - fine_x } else { fine_x };
            let pixel_y = if flip_v { 7 - fine_y } else { fine_y };
            
            let color_index = self.get_tile_pixel(chr_addr, tile_num, pixel_x, pixel_y, bpp);
            
            // Color 0 is transparent
            if color_index != 0 {
                let palette_index = (palette << bpp) | color_index;
                let color = self.cgram[palette_index as usize];
                
                // Only draw if higher priority
                if layer_priority >= priority[x] {
                    scanline[x] = self.rgb555_to_rgba8888(color);
                    priority[x] = layer_priority;
                }
            }
        }
    }
    
    /// Get bits per pixel for a background layer in a given mode
    fn get_bg_bpp(&self, bg_num: usize, mode: u8) -> u8 {
        match mode {
            0 => 2,  // All layers 2bpp
            1 => if bg_num <= 1 { 4 } else { 2 },  // BG1/2 = 4bpp, BG3 = 2bpp
            2 => 4,  // BG1/2 = 4bpp
            3 => if bg_num == 0 { 8 } else { 4 },  // BG1 = 8bpp, BG2 = 4bpp
            4 => if bg_num == 0 { 8 } else { 2 },  // BG1 = 8bpp, BG2 = 2bpp
            5 => if bg_num == 0 { 4 } else { 2 },  // BG1 = 4bpp, BG2 = 2bpp
            6 => 4,  // BG1 = 4bpp
            7 => 8,  // BG1 = 8bpp (Mode 7)
            _ => 2,
        }
    }
    
    /// Check if background layer uses 16x16 tiles
    fn is_bg_16x16(&self, bg_num: usize) -> bool {
        let bit = match bg_num {
            0 => 0x10,
            1 => 0x20,
            2 => 0x40,
            3 => 0x80,
            _ => 0,
        };
        (self.bgmode & bit) != 0
    }
    
    /// Get a pixel from a tile with the given bpp
    fn get_tile_pixel(&self, chr_addr: u16, tile_num: u16, x: usize, y: usize, bpp: u8) -> u8 {
        match bpp {
            2 => self.decode_2bpp(chr_addr, tile_num, x, y),
            4 => self.decode_4bpp(chr_addr, tile_num, x, y),
            8 => self.decode_8bpp(chr_addr, tile_num, x, y),
            _ => 0,
        }
    }
    
    /// Decode 2bpp tile pixel
    fn decode_2bpp(&self, chr_addr: u16, tile_num: u16, x: usize, y: usize) -> u8 {
        let tile_addr = chr_addr + (tile_num * 16) + (y * 2) as u16;
        let plane0 = self.read_vram(tile_addr);
        let plane1 = self.read_vram(tile_addr + 1);
        
        let bit = 7 - x;
        let bit0 = (plane0 >> bit) & 1;
        let bit1 = (plane1 >> bit) & 1;
        
        bit0 | (bit1 << 1)
    }
    
    /// Decode 4bpp tile pixel
    fn decode_4bpp(&self, chr_addr: u16, tile_num: u16, x: usize, y: usize) -> u8 {
        let tile_addr = chr_addr + (tile_num * 32) + (y * 2) as u16;
        let plane0 = self.read_vram(tile_addr);
        let plane1 = self.read_vram(tile_addr + 1);
        let plane2 = self.read_vram(tile_addr + 16);
        let plane3 = self.read_vram(tile_addr + 17);
        
        let bit = 7 - x;
        let bit0 = (plane0 >> bit) & 1;
        let bit1 = (plane1 >> bit) & 1;
        let bit2 = (plane2 >> bit) & 1;
        let bit3 = (plane3 >> bit) & 1;
        
        bit0 | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
    }
    
    /// Decode 8bpp tile pixel
    fn decode_8bpp(&self, chr_addr: u16, tile_num: u16, x: usize, y: usize) -> u8 {
        let tile_addr = chr_addr + (tile_num * 64) + (y * 2) as u16;
        let plane0 = self.read_vram(tile_addr);
        let plane1 = self.read_vram(tile_addr + 1);
        let plane2 = self.read_vram(tile_addr + 16);
        let plane3 = self.read_vram(tile_addr + 17);
        let plane4 = self.read_vram(tile_addr + 32);
        let plane5 = self.read_vram(tile_addr + 33);
        let plane6 = self.read_vram(tile_addr + 48);
        let plane7 = self.read_vram(tile_addr + 49);
        
        let bit = 7 - x;
        let bit0 = (plane0 >> bit) & 1;
        let bit1 = (plane1 >> bit) & 1;
        let bit2 = (plane2 >> bit) & 1;
        let bit3 = (plane3 >> bit) & 1;
        let bit4 = (plane4 >> bit) & 1;
        let bit5 = (plane5 >> bit) & 1;
        let bit6 = (plane6 >> bit) & 1;
        let bit7 = (plane7 >> bit) & 1;
        
        bit0 | (bit1 << 1) | (bit2 << 2) | (bit3 << 3) |
        (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7)
    }
    
    /// Render sprites
    fn render_sprites(&mut self, scanline: &mut [u32; 512], priority: &mut [u8; 512]) {
        let y = self.scanline as i16;
        
        // Get sprite sizes from OBSEL register
        let (small_size, large_size) = self.get_sprite_sizes();
        let sprite_base = ((self.obsel & 0x07) as u16) << 13;  // Sprite tile base address
        let sprite_name_offset = (((self.obsel >> 3) & 0x03) as u16) << 12;
        
        // Process sprites in reverse order (lower priority rendered first)
        for sprite_idx in (0..128).rev() {
            let oam_offset = sprite_idx * 4;
            
            // Read OAM entry
            let x_low = self.oam[oam_offset] as i16;
            let sprite_y = self.oam[oam_offset + 1];
            let tile = self.oam[oam_offset + 2] as u16;
            let attr = self.oam[oam_offset + 3];
            
            // Read high table for X MSB and size
            let high_byte = self.oam[512 + (sprite_idx >> 2)];
            let high_shift = (sprite_idx & 3) * 2;
            let x_msb = ((high_byte >> high_shift) & 0x01) as i16;
            let size_bit = ((high_byte >> (high_shift + 1)) & 0x01) != 0;
            
            let x = x_low | (x_msb << 8);
            let x = if x >= 256 { x - 512 } else { x };  // Sign extend
            
            let palette = ((attr >> 1) & 0x07) + 8;  // Sprite palettes are 128-255
            let sprite_priority = (attr >> 4) & 0x03;
            let flip_h = (attr & 0x40) != 0;
            let flip_v = (attr & 0x80) != 0;
            
            let (width, height) = if size_bit { large_size } else { small_size };
            
            // Check if sprite intersects this scanline
            let sprite_top = sprite_y as i16;
            let sprite_bottom = sprite_top + height as i16;
            
            if y < sprite_top || y >= sprite_bottom {
                continue;
            }
            
            let row = (y - sprite_top) as u8;
            let tile_row = if flip_v { height - 1 - row } else { row };
            
            // Render sprite pixels
            for col in 0..width {
                let screen_x = x + col as i16;
                if screen_x < 0 || screen_x >= 256 {
                    continue;
                }
                
                let tile_col = if flip_h { width - 1 - col } else { col };
                
                // Calculate tile address
                let tile_x = tile_col >> 3;
                let tile_y = tile_row >> 3;
                let tiles_per_row = width >> 3;
                let tile_offset = tile_y * tiles_per_row + tile_x;
                let current_tile = tile + tile_offset as u16;
                
                let pixel_x = (tile_col & 7) as usize;
                let pixel_y = (tile_row & 7) as usize;
                
                // Sprites are always 4bpp
                let tile_addr = sprite_base + sprite_name_offset + (current_tile * 32);
                let color_index = self.decode_4bpp(tile_addr, 0, pixel_x, pixel_y);
                
                if color_index != 0 {
                    let palette_index = (palette << 4) | color_index;
                    let color = self.cgram[palette_index as usize];
                    
                    // Sprite priorities: 0-3, mapped to layer priorities
                    let sprite_layer_priority = 12 + sprite_priority;
                    
                    if sprite_layer_priority >= priority[screen_x as usize] {
                        scanline[screen_x as usize] = self.rgb555_to_rgba8888(color);
                        priority[screen_x as usize] = sprite_layer_priority;
                    }
                }
            }
        }
    }
    
    /// Get sprite sizes based on OBSEL register
    fn get_sprite_sizes(&self) -> ((u8, u8), (u8, u8)) {
        match self.obsel >> 5 {
            0 => ((8, 8), (16, 16)),
            1 => ((8, 8), (32, 32)),
            2 => ((8, 8), (64, 64)),
            3 => ((16, 16), (32, 32)),
            4 => ((16, 16), (64, 64)),
            5 => ((32, 32), (64, 64)),
            6 => ((16, 32), (32, 64)),
            7 => ((16, 32), (32, 32)),
            _ => ((8, 8), (16, 16)),
        }
    }
    
    /// Convert RGB555 to RGBA8888
    fn rgb555_to_rgba8888(&self, color: u16) -> u32 {
        let r = (color & 0x1F) as u8;
        let g = ((color >> 5) & 0x1F) as u8;
        let b = ((color >> 10) & 0x1F) as u8;
        
        // Convert 5-bit to 8-bit (scale 0-31 to 0-255)
        let r8 = (r << 3) | (r >> 2);
        let g8 = (g << 3) | (g >> 2);
        let b8 = (b << 3) | (b >> 2);
        
        (0xFF << 24) | ((b8 as u32) << 16) | ((g8 as u32) << 8) | (r8 as u32)
    }
    
    /// Apply brightness adjustment
    fn apply_brightness(&self, color: u32, brightness: u8) -> u32 {
        let r = (color & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = ((color >> 16) & 0xFF) as u8;
        let a = ((color >> 24) & 0xFF) as u8;
        
        let r_adj = ((r as u16 * brightness as u16) / 15) as u8;
        let g_adj = ((g as u16 * brightness as u16) / 15) as u8;
        let b_adj = ((b as u16 * brightness as u16) / 15) as u8;
        
        ((a as u32) << 24) | ((b_adj as u32) << 16) | ((g_adj as u32) << 8) | (r_adj as u32)
    }
    
    /// Read from VRAM
    fn read_vram(&self, addr: u16) -> u8 {
        self.vram[addr as usize]
    }
    
    /// Write to VRAM
    fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[addr as usize] = value;
    }
    
    // PPU Register Read/Write Functions
    
    /// Write to PPU register
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x2100 => self.inidisp = value,
            0x2101 => self.obsel = value,
            0x2102 => {
                self.oamadd = (self.oamadd & 0xFF00) | value as u16;
                self.oam_address = self.oamadd;
            },
            0x2103 => {
                self.oamadd = (self.oamadd & 0x00FF) | ((value as u16 & 0x01) << 8);
                self.oam_address = self.oamadd;
                self.oam_high_byte = false;
            },
            0x2104 => {
                // OAM data write
                if self.oam_address < 544 {
                    self.oam[self.oam_address as usize] = value;
                    self.oam_address = (self.oam_address + 1) & 0x21F;
                }
            },
            0x2105 => self.bgmode = value,
            0x2106 => self.mosaic = value,
            0x2107 => {
                // BG1 tilemap address and size
                self.bg1_tilemap_addr = ((value as u16 & 0xFC) >> 2) << 11;
            },
            0x2108 => {
                // BG2 tilemap address and size
                self.bg2_tilemap_addr = ((value as u16 & 0xFC) >> 2) << 11;
            },
            0x2109 => {
                // BG3 tilemap address and size
                self.bg3_tilemap_addr = ((value as u16 & 0xFC) >> 2) << 11;
            },
            0x210A => {
                // BG4 tilemap address and size
                self.bg4_tilemap_addr = ((value as u16 & 0xFC) >> 2) << 11;
            },
            0x210B => {
                // BG1 and BG2 character data address
                self.bg1_chr_addr = (value as u16 & 0x0F) << 13;
                self.bg2_chr_addr = (value as u16 & 0xF0) << 9;
            },
            0x210C => {
                // BG3 and BG4 character data address
                self.bg3_chr_addr = (value as u16 & 0x0F) << 13;
                self.bg4_chr_addr = (value as u16 & 0xF0) << 9;
            },
            0x210D => {
                // BG1 horizontal scroll
                let prev = self.bg_scroll_latch;
                self.bg1_hscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x210E => {
                // BG1 vertical scroll
                let prev = self.bg_scroll_latch;
                self.bg1_vscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x210F => {
                // BG2 horizontal scroll
                let prev = self.bg_scroll_latch;
                self.bg2_hscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2110 => {
                // BG2 vertical scroll
                let prev = self.bg_scroll_latch;
                self.bg2_vscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2111 => {
                // BG3 horizontal scroll
                let prev = self.bg_scroll_latch;
                self.bg3_hscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2112 => {
                // BG3 vertical scroll
                let prev = self.bg_scroll_latch;
                self.bg3_vscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2113 => {
                // BG4 horizontal scroll
                let prev = self.bg_scroll_latch;
                self.bg4_hscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2114 => {
                // BG4 vertical scroll
                let prev = self.bg_scroll_latch;
                self.bg4_vscroll = ((value as u16) << 8) | (prev as u16);
                self.bg_scroll_latch = value;
            },
            0x2115 => {
                // VRAM address increment mode
                self.vram_increment = match value & 0x03 {
                    0 => 1,
                    1 => 32,
                    2 | 3 => 128,
                    _ => 1,
                };
                self.vram_mapping = if (value & 0x04) != 0 {
                    VramMapping::Increment1
                } else {
                    VramMapping::Increment0
                };
            },
            0x2116 => {
                // VRAM address low
                self.vram_address = (self.vram_address & 0xFF00) | value as u16;
            },
            0x2117 => {
                // VRAM address high
                self.vram_address = (self.vram_address & 0x00FF) | ((value as u16) << 8);
            },
            0x2118 => {
                // VRAM data write low
                let addr = (self.vram_address as usize) * 2;
                if addr < 0x10000 {
                    self.write_vram(addr as u16, value);
                }
                if matches!(self.vram_mapping, VramMapping::Increment0) {
                    self.vram_address = self.vram_address.wrapping_add(self.vram_increment);
                }
            },
            0x2119 => {
                // VRAM data write high
                let addr = (self.vram_address as usize) * 2 + 1;
                if addr < 0x10000 {
                    self.write_vram(addr as u16, value);
                }
                if matches!(self.vram_mapping, VramMapping::Increment1) {
                    self.vram_address = self.vram_address.wrapping_add(self.vram_increment);
                }
            },
            0x211A => self.m7sel = value,
            0x211B => {
                // Mode 7 matrix A
                let prev = self.m7_latch;
                self.m7a = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x211C => {
                // Mode 7 matrix B
                let prev = self.m7_latch;
                self.m7b = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x211D => {
                // Mode 7 matrix C
                let prev = self.m7_latch;
                self.m7c = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x211E => {
                // Mode 7 matrix D
                let prev = self.m7_latch;
                self.m7d = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x211F => {
                // Mode 7 center X
                let prev = self.m7_latch;
                self.m7x = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x2120 => {
                // Mode 7 center Y
                let prev = self.m7_latch;
                self.m7y = ((value as i16) << 8) | (prev as i16);
                self.m7_latch = value;
            },
            0x2121 => {
                // CGRAM address
                self.cgram_address = value;
                self.cgram_high_byte = false;
            },
            0x2122 => {
                // CGRAM data write
                if !self.cgram_high_byte {
                    self.cgram_latch = value;
                    self.cgram_high_byte = true;
                } else {
                    let color = ((value as u16) << 8) | (self.cgram_latch as u16);
                    self.cgram[self.cgram_address as usize] = color;
                    self.cgram_address = self.cgram_address.wrapping_add(1);
                    self.cgram_high_byte = false;
                }
            },
            0x2123..=0x2125 => {
                // Window mask settings for BG1-4 and OBJ
                let idx = (addr - 0x2123) as usize;
                if idx < 3 {
                    self.window_mask_settings[idx * 2] = value & 0x0F;
                    self.window_mask_settings[idx * 2 + 1] = (value >> 4) & 0x0F;
                }
            },
            0x2126 => self.window1_left = value,
            0x2127 => self.window1_right = value,
            0x2128 => self.window2_left = value,
            0x2129 => self.window2_right = value,
            0x212A => {
                // Window mask logic for BG1-4
                self.window_mask_logic[0] = value & 0x03;
                self.window_mask_logic[1] = (value >> 2) & 0x03;
                self.window_mask_logic[2] = (value >> 4) & 0x03;
                self.window_mask_logic[3] = (value >> 6) & 0x03;
            },
            0x212B => {
                // Window mask logic for OBJ and Color
                self.window_mask_logic[4] = value & 0x03;
                self.window_mask_logic[5] = (value >> 2) & 0x03;
            },
            0x212C => self.tm = value,   // Main screen designation
            0x212D => self.ts = value,   // Sub screen designation
            0x212E => self.tmw = value,  // Window mask for main screen
            0x212F => self.tsw = value,  // Window mask for sub screen
            0x2130 => self.cgwsel = value,  // Color math control
            0x2131 => self.cgadsub = value,  // Color math designation
            0x2132 => {
                // Fixed color data
                self.coldata = value;
                if value & 0x20 != 0 {
                    self.fixed_color[0] = value & 0x1F;  // R
                }
                if value & 0x40 != 0 {
                    self.fixed_color[1] = value & 0x1F;  // G
                }
                if value & 0x80 != 0 {
                    self.fixed_color[2] = value & 0x1F;  // B
                }
            },
            _ => {
                // Unimplemented register
            }
        }
    }
    
    /// Read from PPU register
    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x2134 => 0,  // MPYL - Multiplication result low
            0x2135 => 0,  // MPYM - Multiplication result middle
            0x2136 => 0,  // MPYH - Multiplication result high
            0x2137 => {
                // SLHV - Software latch for H/V counters
                self.ophct = self.dot;
                self.opvct = self.scanline;
                0
            },
            0x2138 => {
                // OAMDATAREAD - OAM data read
                let value = if self.oam_address < 544 {
                    self.oam[self.oam_address as usize]
                } else {
                    0
                };
                self.oam_address = (self.oam_address + 1) & 0x21F;
                value
            },
            0x2139 => {
                // VMDATALREAD - VRAM data read low
                let addr = (self.vram_address as usize) * 2;
                let value = if addr < 0x10000 {
                    self.read_vram(addr as u16)
                } else {
                    0
                };
                if matches!(self.vram_mapping, VramMapping::Increment0) {
                    self.vram_address = self.vram_address.wrapping_add(self.vram_increment);
                }
                value
            },
            0x213A => {
                // VMDATAHREAD - VRAM data read high
                let addr = (self.vram_address as usize) * 2 + 1;
                let value = if addr < 0x10000 {
                    self.read_vram(addr as u16)
                } else {
                    0
                };
                if matches!(self.vram_mapping, VramMapping::Increment1) {
                    self.vram_address = self.vram_address.wrapping_add(self.vram_increment);
                }
                value
            },
            0x213B => {
                // CGDATAREAD - CGRAM data read
                let color = self.cgram[self.cgram_address as usize];
                let value = if !self.cgram_high_byte {
                    self.cgram_high_byte = true;
                    (color & 0xFF) as u8
                } else {
                    self.cgram_address = self.cgram_address.wrapping_add(1);
                    self.cgram_high_byte = false;
                    ((color >> 8) & 0xFF) as u8
                };
                value
            },
            0x213C => {
                // OPHCT - Horizontal counter latch
                let value = (self.ophct & 0xFF) as u8;
                value
            },
            0x213D => {
                // OPVCT - Vertical counter latch
                let value = (self.opvct & 0xFF) as u8;
                value
            },
            0x213E => {
                // STAT77 - PPU status flag and version
                // Bit 7: Time Over Flag, Bit 6: Range Over Flag
                // Bits 0-4: PPU1 version (5C77)
                0x01
            },
            0x213F => {
                // STAT78 - PPU status flag and version
                // Bit 7: Interlace field, Bit 6: External latch
                // Bits 0-4: PPU2 version (5C78)
                let mut value = 0x03;
                if self.vblank {
                    value |= 0x80;
                }
                if self.hblank {
                    value |= 0x40;
                }
                value
            },
            _ => 0,
        }
    }
    
    /// Check if in VBlank
    pub fn in_vblank(&self) -> bool {
        self.vblank
    }
    
    /// Check if in HBlank
    pub fn in_hblank(&self) -> bool {
        self.hblank
    }
    
    /// Get current scanline
    pub fn get_scanline(&self) -> u16 {
        self.scanline
    }
    
    /// Reset PPU state
    pub fn reset(&mut self) {
        self.scanline = 0;
        self.dot = 0;
        self.vblank = true;
        self.hblank = false;
        self.frame_complete = false;
        self.inidisp = 0x80;  // Force blank
        
        // Clear framebuffer to black
        for pixel in self.framebuffer.iter_mut() {
            *pixel = 0xFF000000;  // Black with full alpha
        }
    }
}

// Helper methods for bulk loading (used by emulator)
impl Ppu {
    /// Write data to VRAM (for bulk loading)
    pub fn write_vram_wasm(&mut self, addr: u16, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let vram_addr = addr.wrapping_add(i as u16);
            if (vram_addr as usize) < self.vram.len() {
                self.vram[vram_addr as usize] = byte;
            }
        }
    }
    
    /// Write data to CGRAM (for bulk palette loading)
    pub fn write_cgram_wasm(&mut self, addr: u8, data: &[u16]) {
        for (i, &color) in data.iter().enumerate() {
            let cgram_addr = addr.wrapping_add(i as u8);
            if (cgram_addr as usize) < self.cgram.len() {
                self.cgram[cgram_addr as usize] = color;
            }
        }
    }
    
    /// Write data to OAM (for bulk sprite loading)
    pub fn write_oam_wasm(&mut self, addr: u16, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let oam_addr = addr.wrapping_add(i as u16);
            if (oam_addr as usize) < self.oam.len() {
                self.oam[oam_addr as usize] = byte;
            }
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}
