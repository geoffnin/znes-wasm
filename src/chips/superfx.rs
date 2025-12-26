/// SuperFX (GSU) Coprocessor Implementation
///
/// The SuperFX/GSU (Graphics Support Unit) is a RISC-style coprocessor designed for
/// 3D graphics and sprite manipulation. It features:
/// - 16 general-purpose registers (R0-R15)
/// - 512-byte cache RAM
/// - Fast pixel plotting
/// - Variable clock speeds (10.7 MHz or 21.4 MHz)
/// - ROM/RAM banking
///
/// Used in games like Star Fox, Yoshi's Island, and Doom.
///
/// Memory Map:
/// - 0x3000-0x301F: SuperFX Registers
/// - 0x3030-0x3033: Additional control registers
/// - 0x3100-0x32FF: Cache RAM (512 bytes)

use super::CoProcessor;

/// SuperFX Instruction Opcodes (partial set)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
enum SuperFxOp {
    // ALU Operations
    Add = 0x50,      // ADD Rn - Add to R0
    Sub = 0x60,      // SUB Rn - Subtract from R0
    Mult = 0x80,     // MULT Rn - Multiply
    
    // Load/Store
    Ldb = 0x40,      // LDB (Rn) - Load byte
    Stb = 0x30,      // STB (Rn) - Store byte
    Ldw = 0x41,      // LDW (Rn) - Load word
    Stw = 0x31,      // STW (Rn) - Store word
    
    // Immediate values
    Ibt = 0xA0,      // IBT Rn,#n - Immediate byte to register
    Iwt = 0xF0,      // IWT Rn,#nn - Immediate word to register
    
    // Register operations
    To = 0x10,       // TO Rn - Set destination register
    From = 0xB0,     // FROM Rn - Copy from source register
    
    // Pixel operations
    Plot = 0x4C,     // PLOT - Plot pixel
    Color = 0x4E,    // COLOR - Set color
    GetC = 0x20,     // GETC - Read ROM byte
    RamB = 0xDF,     // RAMB - Set RAM bank
    RomB = 0xDE,     // ROMB - Set ROM bank
    
    // Control flow
    Stop = 0x00,     // STOP - Stop execution
    Nop = 0x01,      // NOP - No operation
    Cache = 0x02,    // CACHE - Flush cache
    Bra = 0x05,      // BRA - Branch always
    
    // Status operations
    GetB = 0xEF,     // GETB - Get byte from ROM buffer
    GetBh = 0xF6,    // GETBH - Get byte high
    GetBl = 0xF7,    // GETBL - Get byte low
    GetBs = 0xF8,    // GETBS - Get byte signed
}

/// SuperFX Status Register Flags
#[derive(Debug, Default)]
struct SfxStatus {
    go: bool,           // Running flag
    irq: bool,          // IRQ flag
    carry: bool,        // Carry flag
    zero: bool,         // Zero flag
    sign: bool,         // Sign flag
    overflow: bool,     // Overflow flag
}

/// SuperFX Coprocessor State
pub struct SuperFx {
    /// 16 general-purpose registers (R0-R15)
    /// R0: Default destination/accumulator
    /// R1-R13: General purpose
    /// R14: ROM buffer pointer
    /// R15: Program counter
    r: [u16; 16],
    
    /// 512-byte cache RAM
    cache: Box<[u8; 0x200]>,
    
    /// Status flags
    status: SfxStatus,
    
    /// Screen buffer settings
    screen_base: u16,       // Screen buffer base address
    screen_height: u8,      // Screen height in pixels
    
    /// Banking registers
    rom_bank: u8,           // ROM bank register
    ram_bank: u8,           // RAM bank register
    
    /// Cache settings
    cache_base: u16,        // Cache base register
    
    /// Control registers
    cfgr: u8,               // Configuration register (clock speed, etc.)
    scbr: u8,               // Screen base register
    clsr: u8,               // Clock select register
    por: u8,                // Plot option register
    
    /// Color register for pixel plotting
    color_reg: u8,
    
    /// Pixel cache for fast plotting
    plot_transparent: bool,
    plot_dither: bool,
    plot_high: bool,
    
    /// Cycle counter
    cycles: u64,
    
    /// Clock speed multiplier (1 = 10.7 MHz, 2 = 21.4 MHz)
    clock_multiplier: u32,
}

impl SuperFx {
    pub fn new() -> Self {
        Self {
            r: [0; 16],
            cache: Box::new([0; 0x200]),
            status: SfxStatus::default(),
            screen_base: 0,
            screen_height: 128,
            rom_bank: 0,
            ram_bank: 0,
            cache_base: 0,
            cfgr: 0,
            scbr: 0,
            clsr: 0,
            por: 0,
            color_reg: 0,
            plot_transparent: false,
            plot_dither: false,
            plot_high: false,
            cycles: 0,
            clock_multiplier: 1,
        }
    }

    /// Get program counter (R15)
    #[inline]
    fn pc(&self) -> u16 {
        self.r[15]
    }

    /// Set program counter (R15)
    #[inline]
    fn set_pc(&mut self, val: u16) {
        self.r[15] = val;
    }

    /// Get source/destination register (R12)
    #[inline]
    #[allow(dead_code)]
    fn sreg(&self) -> u16 {
        self.r[12]
    }

    /// Get destination register (R13)
    #[inline]
    #[allow(dead_code)]
    fn dreg(&self) -> u16 {
        self.r[13]
    }

    /// Update zero and sign flags based on value
    fn update_flags(&mut self, value: u16) {
        self.status.zero = value == 0;
        self.status.sign = (value & 0x8000) != 0;
    }

    /// Execute a single SuperFX instruction
    fn execute_instruction(&mut self, opcode: u8) -> u32 {
        // Decode instruction (simplified - real SuperFX has complex encoding)
        let op_type = opcode & 0xF0;
        let operand = opcode & 0x0F;
        
        match op_type {
            // Stop
            0x00 if opcode == 0x00 => {
                self.status.go = false;
                return 1;
            }
            
            // NOP
            0x00 if opcode == 0x01 => {
                return 1;
            }
            
            // MOVE operations (0x10-0x1F)
            0x10 => {
                // TO Rn - Set destination register
                self.r[13] = operand as u16;
                return 1;
            }
            
            // STB (0x30-0x3F) - Store byte
            0x30 => {
                let addr = self.r[operand as usize];
                let value = self.r[0] as u8;
                // In real implementation, would write to RAM/cache
                if (addr as usize) < 0x200 {
                    self.cache[addr as usize] = value;
                }
                return 1;
            }
            
            // LDB (0x40-0x4F) - Load byte
            0x40 => {
                let addr = self.r[operand as usize];
                let value = if (addr as usize) < 0x200 {
                    self.cache[addr as usize]
                } else {
                    0
                };
                self.r[0] = value as u16;
                self.update_flags(self.r[0]);
                return 1;
            }
            
            // ADD (0x50-0x5F) - Add to R0
            0x50 => {
                let result = self.r[0].wrapping_add(self.r[operand as usize]);
                self.status.carry = result < self.r[0];
                self.status.overflow = 
                    ((self.r[0] ^ self.r[operand as usize]) & 0x8000) == 0 &&
                    ((self.r[0] ^ result) & 0x8000) != 0;
                self.r[0] = result;
                self.update_flags(result);
                return 1;
            }
            
            // SUB (0x60-0x6F) - Subtract from R0
            0x60 => {
                let result = self.r[0].wrapping_sub(self.r[operand as usize]);
                self.status.carry = self.r[0] >= self.r[operand as usize];
                self.status.overflow = 
                    ((self.r[0] ^ self.r[operand as usize]) & 0x8000) != 0 &&
                    ((self.r[0] ^ result) & 0x8000) != 0;
                self.r[0] = result;
                self.update_flags(result);
                return 1;
            }
            
            // MULT (0x80-0x8F) - Multiply
            0x80 => {
                let result = (self.r[0] as i16 as i32) * (self.r[operand as usize] as i16 as i32);
                self.r[0] = result as u16;
                self.update_flags(self.r[0]);
                return 8; // Multiply takes more cycles
            }
            
            // IBT (0xA0-0xAF) - Immediate byte to register
            0xA0 => {
                // Next byte is the immediate value
                let imm = self.fetch_byte();
                self.r[operand as usize] = (imm as i8) as i16 as u16; // Sign-extend
                return 2;
            }
            
            // FROM (0xB0-0xBF) - Copy from source register
            0xB0 => {
                self.r[0] = self.r[operand as usize];
                self.update_flags(self.r[0]);
                return 1;
            }
            
            // IWT (0xF0-0xFF) - Immediate word to register
            0xF0 => {
                // Next two bytes are the immediate value
                let lo = self.fetch_byte() as u16;
                let hi = self.fetch_byte() as u16;
                self.r[operand as usize] = (hi << 8) | lo;
                return 3;
            }
            
            _ => {
                // Handle special opcodes
                match opcode {
                    0x4C => self.op_plot(),     // PLOT
                    0x4E => self.op_color(),    // COLOR
                    0x20 => self.op_getc(),     // GETC
                    0xDF => self.op_ramb(),     // RAMB
                    0xDE => self.op_romb(),     // ROMB
                    _ => 1, // Unknown opcode
                }
            }
        }
    }

    /// Fetch a byte from the program (increment PC)
    fn fetch_byte(&mut self) -> u8 {
        let pc = self.pc();
        self.set_pc(pc.wrapping_add(1));
        // In real implementation, would fetch from ROM
        // For now, return from cache if in range
        if (pc as usize) < 0x200 {
            self.cache[pc as usize]
        } else {
            0
        }
    }

    /// PLOT - Plot pixel at current position
    fn op_plot(&mut self) -> u32 {
        let x = self.r[1] & 0xFF;
        let y = self.r[2] & 0xFF;
        let color = self.color_reg;
        
        // Calculate pixel address in screen buffer
        // Simplified: actual SuperFX has complex tile-based addressing
        let addr = self.screen_base.wrapping_add((y as u16) * 256 + x as u16);
        
        // Write to cache (in real implementation, writes to screen buffer)
        if (addr as usize) < 0x200 {
            if !self.plot_transparent || color != 0 {
                self.cache[addr as usize] = color;
            }
        }
        
        1
    }

    /// COLOR - Set color register
    fn op_color(&mut self) -> u32 {
        self.color_reg = self.r[0] as u8;
        1
    }

    /// GETC - Read byte from ROM buffer
    fn op_getc(&mut self) -> u32 {
        let _addr = self.r[14]; // ROM buffer pointer
        // In real implementation, would read from ROM
        self.r[0] = 0; // Placeholder
        self.r[14] = self.r[14].wrapping_add(1);
        6 // GETC takes multiple cycles
    }

    /// RAMB - Set RAM bank
    fn op_ramb(&mut self) -> u32 {
        let imm = self.fetch_byte();
        self.ram_bank = imm;
        1
    }

    /// ROMB - Set ROM bank
    fn op_romb(&mut self) -> u32 {
        let imm = self.fetch_byte();
        self.rom_bank = imm;
        1
    }

    /// Read SuperFX register
    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            // R0-R15 (32 bytes, 2 bytes per register)
            0x3000..=0x301F => {
                let reg_num = ((addr - 0x3000) >> 1) as usize;
                let reg_byte = (addr - 0x3000) & 1;
                if reg_byte == 0 {
                    (self.r[reg_num] & 0xFF) as u8
                } else {
                    ((self.r[reg_num] >> 8) & 0xFF) as u8
                }
            }
            
            // Status register (SFR)
            0x3030 => {
                let mut sfr = 0u8;
                if self.status.go { sfr |= 0x20; }
                if self.status.irq { sfr |= 0x80; }
                if self.status.carry { sfr |= 0x02; }
                if self.status.zero { sfr |= 0x04; }
                if self.status.sign { sfr |= 0x08; }
                if self.status.overflow { sfr |= 0x10; }
                sfr
            }
            
            // Configuration register (CFGR)
            0x3034 => self.cfgr,
            
            // Screen base register
            0x3035 => self.scbr,
            
            // Clock select register
            0x3037 => self.clsr,
            
            // Plot option register
            0x3038 => self.por,
            
            // ROM bank register
            0x3033 => self.rom_bank,
            
            // RAM bank register
            0x303A => self.ram_bank,
            
            _ => 0,
        }
    }

    /// Write SuperFX register
    fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            // R0-R15 (32 bytes, 2 bytes per register)
            0x3000..=0x301F => {
                let reg_num = ((addr - 0x3000) >> 1) as usize;
                let reg_byte = (addr - 0x3000) & 1;
                if reg_byte == 0 {
                    self.r[reg_num] = (self.r[reg_num] & 0xFF00) | (val as u16);
                } else {
                    self.r[reg_num] = (self.r[reg_num] & 0x00FF) | ((val as u16) << 8);
                }
            }
            
            // Status register (SFR) - Write to start/stop
            0x3030 => {
                let old_go = self.status.go;
                self.status.go = (val & 0x20) != 0;
                
                // Starting execution
                if !old_go && self.status.go {
                    // Reset to initial state
                    self.status.irq = false;
                }
                
                // Clear IRQ flag if bit 7 is written
                if val & 0x80 != 0 {
                    self.status.irq = false;
                }
            }
            
            // Configuration register (CFGR)
            0x3034 => {
                self.cfgr = val;
                // Bit 0: IRQ enable
                // Bit 5: High speed mode (21.4 MHz vs 10.7 MHz)
                self.clock_multiplier = if val & 0x20 != 0 { 2 } else { 1 };
            }
            
            // Screen base register
            0x3035 => {
                self.scbr = val;
                self.screen_base = (val as u16) << 9; // Multiply by 512
            }
            
            // Clock select register
            0x3037 => {
                self.clsr = val;
            }
            
            // Plot option register
            0x3038 => {
                self.por = val;
                self.plot_transparent = (val & 0x01) != 0;
                self.plot_dither = (val & 0x02) != 0;
                self.plot_high = (val & 0x10) != 0;
            }
            
            // ROM bank register
            0x3033 => {
                self.rom_bank = val;
            }
            
            // RAM bank register
            0x303A => {
                self.ram_bank = val;
            }
            
            // Color register
            0x303C => {
                self.color_reg = val;
            }
            
            _ => {}
        }
    }

    /// Execute SuperFX for one instruction
    fn execute_step(&mut self) -> u32 {
        if !self.status.go {
            return 0;
        }
        
        let opcode = self.fetch_byte();
        let cycles = self.execute_instruction(opcode);
        
        cycles * self.clock_multiplier
    }
}

impl CoProcessor for SuperFx {
    fn reset(&mut self) {
        self.r = [0; 16];
        self.cache.fill(0);
        self.status = SfxStatus::default();
        self.screen_base = 0;
        self.screen_height = 128;
        self.rom_bank = 0;
        self.ram_bank = 0;
        self.cache_base = 0;
        self.cfgr = 0;
        self.scbr = 0;
        self.clsr = 0;
        self.por = 0;
        self.color_reg = 0;
        self.plot_transparent = false;
        self.plot_dither = false;
        self.plot_high = false;
        self.cycles = 0;
        self.clock_multiplier = 1;
    }

    fn read(&mut self, addr: u32) -> u8 {
        let addr = addr & 0xFFFF;
        
        match addr {
            // SuperFX Registers
            0x3000..=0x303F => self.read_register(addr as u16),
            
            // Cache RAM
            0x3100..=0x32FF => {
                let offset = (addr - 0x3100) as usize;
                self.cache[offset]
            }
            
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, val: u8) {
        let addr = addr & 0xFFFF;
        
        match addr {
            // SuperFX Registers
            0x3000..=0x303F => self.write_register(addr as u16, val),
            
            // Cache RAM
            0x3100..=0x32FF => {
                let offset = (addr - 0x3100) as usize;
                self.cache[offset] = val;
            }
            
            _ => {}
        }
    }

    fn step(&mut self, cycles: u32) -> u32 {
        let mut cycles_executed = 0u32;
        let target_cycles = cycles * self.clock_multiplier;
        
        while cycles_executed < target_cycles && self.status.go {
            let instruction_cycles = self.execute_step();
            cycles_executed += instruction_cycles;
            self.cycles += instruction_cycles as u64;
            
            // Prevent infinite loops
            if cycles_executed > target_cycles * 2 {
                break;
            }
        }
        
        cycles_executed / self.clock_multiplier
    }

    fn handles_address(&self, addr: u32) -> bool {
        let addr = addr & 0xFFFF;
        matches!(addr, 0x3000..=0x303F | 0x3100..=0x32FF)
    }
}

impl Default for SuperFx {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_access() {
        let mut sfx = SuperFx::new();
        
        // Write to R0 (address 0x3000-0x3001)
        sfx.write(0x3000, 0x34);
        sfx.write(0x3001, 0x12);
        
        // Read back
        assert_eq!(sfx.read(0x3000), 0x34);
        assert_eq!(sfx.read(0x3001), 0x12);
        assert_eq!(sfx.r[0], 0x1234);
    }

    #[test]
    fn test_cache_ram() {
        let mut sfx = SuperFx::new();
        
        // Write to cache RAM
        sfx.write(0x3100, 0xAB);
        sfx.write(0x32FF, 0xCD);
        
        // Read back
        assert_eq!(sfx.read(0x3100), 0xAB);
        assert_eq!(sfx.read(0x32FF), 0xCD);
    }

    #[test]
    fn test_add_instruction() {
        let mut sfx = SuperFx::new();
        
        // Set up registers: R0 = 100, R1 = 50
        sfx.r[0] = 100;
        sfx.r[1] = 50;
        
        // Execute ADD R1 (opcode 0x51)
        let cycles = sfx.execute_instruction(0x51);
        
        // Check result
        assert_eq!(sfx.r[0], 150);
        assert!(!sfx.status.zero);
        assert!(!sfx.status.sign);
        assert_eq!(cycles, 1);
    }

    #[test]
    fn test_mult_instruction() {
        let mut sfx = SuperFx::new();
        
        // Set up registers: R0 = 10, R2 = 20
        sfx.r[0] = 10;
        sfx.r[2] = 20;
        
        // Execute MULT R2 (opcode 0x82)
        let cycles = sfx.execute_instruction(0x82);
        
        // Check result
        assert_eq!(sfx.r[0], 200);
        assert_eq!(cycles, 8); // Multiply takes more cycles
    }

    #[test]
    fn test_status_register() {
        let mut sfx = SuperFx::new();
        
        // Initially not running
        assert_eq!(sfx.read(0x3030) & 0x20, 0);
        
        // Start execution
        sfx.write(0x3030, 0x20);
        assert!(sfx.status.go);
        assert_eq!(sfx.read(0x3030) & 0x20, 0x20);
    }

    #[test]
    fn test_clock_speed() {
        let mut sfx = SuperFx::new();
        
        // Default: 10.7 MHz (multiplier = 1)
        assert_eq!(sfx.clock_multiplier, 1);
        
        // Set high-speed mode: 21.4 MHz
        sfx.write(0x3034, 0x20);
        assert_eq!(sfx.clock_multiplier, 2);
    }

    #[test]
    fn test_handles_address() {
        let sfx = SuperFx::new();
        
        assert!(sfx.handles_address(0x3000));
        assert!(sfx.handles_address(0x303F));
        assert!(sfx.handles_address(0x3100));
        assert!(sfx.handles_address(0x32FF));
        assert!(!sfx.handles_address(0x2FFF));
        assert!(!sfx.handles_address(0x3300));
    }
}
