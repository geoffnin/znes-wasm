// Integrated SNES Emulator
// Combines CPU, PPU, and Memory systems

use crate::cpu::Cpu65816;
use crate::apu::Apu;
use crate::ppu::Ppu;
use crate::memory::Memory;
use crate::cartridge::{Cartridge, CartridgeType};
use crate::chips::{ChipType, create_coprocessor};

/// Main SNES Emulator - coordinates CPU, PPU, APU, and memory
pub struct Emulator {
    cpu: Cpu65816,
    ppu: Ppu,
    apu: Apu,
    memory: Option<Memory>,
    cartridge: Option<Cartridge>,
    master_cycles: u64,
    paused: bool,
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> Self {
        Self {
            cpu: Cpu65816::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            memory: None,
            cartridge: None,
            master_cycles: 0,
            paused: false,
        }
    }
    
    /// Reset the entire emulator
    pub fn reset(&mut self) {
        if let Some(ref cart) = self.cartridge {
            // Detect and create coprocessor if needed
            let coprocessor = Self::create_coprocessor_for_cartridge(cart);
            
            let mut memory = Memory::new_with_coprocessor(cart, coprocessor);
            
            // Reset coprocessor if present
            memory.reset_coprocessor();
            
            // Reset CPU with memory
            self.cpu.reset(&mut memory);
            
            self.memory = Some(memory);
        }
        self.ppu.reset();
        self.apu.reset();
        self.master_cycles = 0;
        self.paused = false;
    }
    
    /// Create a coprocessor based on cartridge type
    fn create_coprocessor_for_cartridge(cartridge: &Cartridge) -> Option<Box<dyn crate::chips::CoProcessor>> {
        // Check if cartridge has a coprocessor
        if let CartridgeType::RomCoprocessor(chip_byte) = cartridge.cartridge_type() {
            if let Some(chip_type) = ChipType::from_cartridge_byte(chip_byte) {
                return create_coprocessor(chip_type);
            }
        }
        None
    }
    
    /// Run emulator for one frame (returns true when frame completes)
    pub fn run_frame(&mut self) -> bool {
        if self.paused {
            return false;
        }
        
        let mut frame_complete = false;
        
        // Run until a frame completes
        // SNES master clock: ~21.477 MHz
        // CPU runs at master/6, PPU at master/4 for dots
        // For simplicity, we run them in lockstep
        
        while !frame_complete {
            // Step PPU (runs every master cycle)
            frame_complete = self.ppu.step();
            
            // Step CPU (runs every 6-12 master cycles depending on mode)
            // For now, step once per several PPU cycles
            if self.master_cycles % 6 == 0 {
                self.step_cpu();
                
                // Step coprocessor if present
                if let Some(ref mut memory) = self.memory {
                    memory.step_coprocessor(6);
                }
            }

            // Keep the APU running at roughly 1 MHz (coarse approximation)
            // 1 APU step per master cycle is too slow; batch a few.
            self.apu.step_spc(2);
            
            self.master_cycles += 1;
            
            // Safety limit to prevent infinite loops
            if self.master_cycles % 100000 == 0 {
                break;
            }
        }
        
        frame_complete
    }
    
    /// Step the emulator by one instruction
    pub fn step(&mut self) {
        if !self.paused {
            self.step_cpu();
            
            // Step coprocessor if present
            if let Some(ref mut memory) = self.memory {
                memory.step_coprocessor(6);
            }
            
            // Step PPU proportionally (approximately 6 dots per CPU cycle)
            for _ in 0..6 {
                self.ppu.step();
            }

            // Run a small batch of APU cycles to keep audio logic alive
            self.apu.step_spc(32);
            
            self.master_cycles += 6;
        }
    }
    
    /// Step CPU and handle memory-mapped I/O
    fn step_cpu(&mut self) {
        if let Some(ref mut memory) = self.memory {
            self.cpu.step(memory);
        }
    }
    
    /// Read byte from memory or memory-mapped I/O
    #[allow(dead_code)]
    fn read_byte(&mut self, addr: u32) -> u8 {
        // Check for PPU register reads ($2100-$213F)
        if (0x2100..=0x213F).contains(&(addr as u16)) {
            return self.ppu.read_register(addr as u16);
        }
        
        // Check for APU I/O ports ($2140-$2143)
        if (0x2140..=0x2143).contains(&(addr as u16)) {
            return self.apu.cpu_read_port(addr as u16);
        }
        
        // Otherwise read from main memory (which may include coprocessor)
        if let Some(ref mut memory) = self.memory {
            memory.read(addr)
        } else {
            0
        }
    }
    
    /// Write byte to memory or memory-mapped I/O
    #[allow(dead_code)]
    fn write_byte(&mut self, addr: u32, value: u8) {
        // Check for PPU register writes ($2100-$2133)
        if (0x2100..=0x2133).contains(&(addr as u16)) {
            self.ppu.write_register(addr as u16, value);
            return;
        }
        
        // Check for APU I/O ports ($2140-$2143)
        if (0x2140..=0x2143).contains(&(addr as u16)) {
            self.apu.cpu_write_port(addr as u16, value);
            return;
        }
        
        // Otherwise write to main memory
        if let Some(ref mut memory) = self.memory {
            memory.write(addr, value);
        }
    }
    
    /// Load ROM data into memory
    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<(), String> {
        if rom_data.is_empty() {
            return Err("ROM data is empty".to_string());
        }
        
        // Create cartridge from ROM data
        let cartridge = Cartridge::from_rom(rom_data.to_vec())
            .map_err(|e| format!("Failed to load ROM: {:?}", e))?;
        
        // Detect and create coprocessor if needed
        let coprocessor = Self::create_coprocessor_for_cartridge(&cartridge);
        
        // Create memory system with cartridge and coprocessor
        let mut memory = Memory::new_with_coprocessor(&cartridge, coprocessor);
        
        // Reset CPU with new memory
        self.cpu.reset(&mut memory);
        
        self.cartridge = Some(cartridge);
        self.memory = Some(memory);
        
        Ok(())
    }
    
    /// Get framebuffer as slice
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.ppu.framebuffer
    }
    
    /// Get framebuffer as mutable slice
    pub fn get_framebuffer_mut(&mut self) -> &mut [u32] {
        &mut self.ppu.framebuffer
    }
    
    /// Get framebuffer dimensions
    pub fn get_framebuffer_size(&self) -> (usize, usize) {
        (512, 478)
    }
    
    /// Write to PPU register
    pub fn write_ppu_register(&mut self, addr: u16, value: u8) {
        self.ppu.write_register(addr, value);
    }
    
    /// Read from PPU register
    pub fn read_ppu_register(&mut self, addr: u16) -> u8 {
        self.ppu.read_register(addr)
    }
    
    /// Check if in VBlank
    pub fn in_vblank(&self) -> bool {
        self.ppu.in_vblank()
    }
    
    /// Get current scanline
    pub fn get_scanline(&self) -> u16 {
        self.ppu.get_scanline()
    }
    
    /// Pause emulation
    pub fn pause(&mut self) {
        self.paused = true;
    }
    
    /// Resume emulation
    pub fn resume(&mut self) {
        self.paused = false;
    }
    
    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }
    
    /// Get total master cycles executed
    pub fn get_master_cycles(&self) -> u64 {
        self.master_cycles
    }
    
    /// Direct VRAM write for testing/debugging
    pub fn write_vram(&mut self, addr: u16, data: &[u8]) {
        self.ppu.write_vram_wasm(addr, data);
    }
    
    /// Direct CGRAM write for testing/debugging
    pub fn write_cgram(&mut self, addr: u8, data: &[u16]) {
        self.ppu.write_cgram_wasm(addr, data);
    }
    
    /// Direct OAM write for testing/debugging
    pub fn write_oam(&mut self, addr: u16, data: &[u8]) {
        self.ppu.write_oam_wasm(addr, data);
    }
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

// Non-wasm implementation for native use
impl Emulator {
    /// Get reference to CPU
    pub fn cpu(&self) -> &Cpu65816 {
        &self.cpu
    }
    
    /// Get mutable reference to CPU
    pub fn cpu_mut(&mut self) -> &mut Cpu65816 {
        &mut self.cpu
    }
    
    /// Get reference to PPU
    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }
    
    /// Get mutable reference to PPU
    pub fn ppu_mut(&mut self) -> &mut Ppu {
        &mut self.ppu
    }
    
    /// Get reference to Memory
    pub fn memory(&self) -> Option<&Memory> {
        self.memory.as_ref()
    }
    
    /// Get mutable reference to Memory
    pub fn memory_mut(&mut self) -> Option<&mut Memory> {
        self.memory.as_mut()
    }

    /// Render one 32kHz stereo audio frame (534 samples) from the APU.
    pub fn render_audio_frame(&mut self) -> &[i16] {
        self.apu.render_frame()
    }

    /// Get mutable access to the APU.
    pub fn apu_mut(&mut self) -> &mut Apu {
        &mut self.apu
    }

    /// Get immutable access to the APU.
    pub fn apu(&self) -> &Apu {
        &self.apu
    }
}
