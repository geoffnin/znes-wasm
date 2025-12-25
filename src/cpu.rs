/// 65816 CPU Emulation for SNES
/// 
/// The 65816 is a 16-bit extension of the 6502, supporting both 8-bit and 16-bit operations.
/// It features emulation mode (6502 compatible) and native mode with enhanced capabilities.

use crate::memory::Memory;

/// Main CPU structure
pub struct Cpu65816 {
    /// Accumulator (16-bit, but can operate as 8-bit)
    pub a: u16,
    
    /// X Index Register (16-bit, but can operate as 8-bit)
    pub x: u16,
    
    /// Y Index Register (16-bit, but can operate as 8-bit)
    pub y: u16,
    
    /// Stack Pointer (16-bit)
    pub s: u16,
    
    /// Direct Page Register (16-bit)
    pub d: u16,
    
    /// Program Counter (16-bit)
    pub pc: u16,
    
    /// Program Bank Register (8-bit)
    pub pbr: u8,
    
    /// Data Bank Register (8-bit)
    pub dbr: u8,
    
    /// Processor Status Flags
    pub p: StatusFlags,
    
    /// Cycle counter
    pub cycles: u64,
    
    /// Flag for stopped state
    pub stopped: bool,
    
    /// Flag for waiting for interrupt
    pub waiting: bool,
}

/// Processor Status Flags
#[derive(Debug, Clone, Copy)]
pub struct StatusFlags {
    /// Negative flag
    pub n: bool,
    
    /// Overflow flag
    pub v: bool,
    
    /// Accumulator register size (0 = 16-bit, 1 = 8-bit) - Native mode only
    pub m: bool,
    
    /// Index register size (0 = 16-bit, 1 = 8-bit) - Native mode only
    pub x: bool,
    
    /// Decimal mode flag
    pub d: bool,
    
    /// IRQ disable flag
    pub i: bool,
    
    /// Zero flag
    pub z: bool,
    
    /// Carry flag
    pub c: bool,
    
    /// Emulation mode flag (1 = 6502 emulation, 0 = native 65816)
    pub e: bool,
}

impl StatusFlags {
    pub fn new() -> Self {
        StatusFlags {
            n: false,
            v: false,
            m: true,  // Start in 8-bit mode
            x: true,  // Start in 8-bit mode
            d: false,
            i: true,  // IRQs disabled at startup
            z: false,
            c: false,
            e: true,  // Start in emulation mode
        }
    }
    
    /// Convert flags to byte (for stack operations)
    pub fn to_byte(&self) -> u8 {
        (if self.n { 0x80 } else { 0 }) |
        (if self.v { 0x40 } else { 0 }) |
        (if self.m { 0x20 } else { 0 }) |
        (if self.x { 0x10 } else { 0 }) |
        (if self.d { 0x08 } else { 0 }) |
        (if self.i { 0x04 } else { 0 }) |
        (if self.z { 0x02 } else { 0 }) |
        (if self.c { 0x01 } else { 0 })
    }
    
    /// Load flags from byte (for stack operations)
    pub fn from_byte(&mut self, value: u8) {
        self.n = (value & 0x80) != 0;
        self.v = (value & 0x40) != 0;
        self.m = (value & 0x20) != 0;
        self.x = (value & 0x10) != 0;
        self.d = (value & 0x08) != 0;
        self.i = (value & 0x04) != 0;
        self.z = (value & 0x02) != 0;
        self.c = (value & 0x01) != 0;
    }
}

impl Default for StatusFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu65816 {
    /// Create a new CPU instance
    pub fn new() -> Self {
        Cpu65816 {
            a: 0,
            x: 0,
            y: 0,
            s: 0x01FF,  // Stack starts at $01FF in emulation mode
            d: 0,
            pc: 0,
            pbr: 0,
            dbr: 0,
            p: StatusFlags::new(),
            cycles: 0,
            stopped: false,
            waiting: false,
        }
    }
    
    /// Reset the CPU
    pub fn reset(&mut self, memory: &Memory) {
        // Read reset vector from $00FFFC-$00FFFD
        let pcl = memory.read(0x00FFFC) as u16;
        let pch = memory.read(0x00FFFD) as u16;
        self.pc = pcl | (pch << 8);
        
        self.pbr = 0;
        self.dbr = 0;
        self.d = 0;
        self.s = 0x01FF;
        
        // Enter emulation mode
        self.p.e = true;
        self.p.m = true;
        self.p.x = true;
        self.p.i = true;
        self.p.d = false;
        
        self.cycles = 0;
        self.stopped = false;
        self.waiting = false;
    }
    
    /// Execute one instruction
    pub fn step(&mut self, memory: &mut Memory) -> u8 {
        if self.stopped {
            return 1;
        }
        
        if self.waiting {
            // TODO: Check for interrupts
            return 1;
        }
        
        let opcode = self.fetch_byte(memory);
        let cycles = self.execute_opcode(opcode, memory);
        self.cycles += cycles as u64;
        cycles
    }
    
    /// Fetch a byte from current PC and increment
    #[inline]
    fn fetch_byte(&mut self, memory: &Memory) -> u8 {
        let addr = ((self.pbr as u32) << 16) | (self.pc as u32);
        let value = memory.read(addr);
        self.pc = self.pc.wrapping_add(1);
        value
    }
    
    /// Fetch a 16-bit word from current PC and increment
    #[inline]
    fn fetch_word(&mut self, memory: &Memory) -> u16 {
        let lo = self.fetch_byte(memory) as u16;
        let hi = self.fetch_byte(memory) as u16;
        lo | (hi << 8)
    }
    
    /// Update N and Z flags based on 8-bit value
    #[inline]
    fn update_nz_8(&mut self, value: u8) {
        self.p.n = (value & 0x80) != 0;
        self.p.z = value == 0;
    }
    
    /// Update N and Z flags based on 16-bit value
    #[inline]
    fn update_nz_16(&mut self, value: u16) {
        self.p.n = (value & 0x8000) != 0;
        self.p.z = value == 0;
    }
    
    /// Push byte to stack
    #[inline]
    fn push_byte(&mut self, memory: &mut Memory, value: u8) {
        let addr = if self.p.e {
            // Emulation mode: stack in page 1
            0x0100 | (self.s & 0xFF) as u32
        } else {
            // Native mode: full 16-bit stack pointer
            self.s as u32
        };
        memory.write(addr, value);
        self.s = self.s.wrapping_sub(1);
        if self.p.e {
            self.s = 0x0100 | (self.s & 0xFF);
        }
    }
    
    /// Push word to stack
    #[inline]
    fn push_word(&mut self, memory: &mut Memory, value: u16) {
        self.push_byte(memory, (value >> 8) as u8);
        self.push_byte(memory, (value & 0xFF) as u8);
    }
    
    /// Pull byte from stack
    #[inline]
    fn pull_byte(&mut self, memory: &Memory) -> u8 {
        self.s = self.s.wrapping_add(1);
        if self.p.e {
            self.s = 0x0100 | (self.s & 0xFF);
        }
        let addr = if self.p.e {
            0x0100 | (self.s & 0xFF) as u32
        } else {
            self.s as u32
        };
        memory.read(addr)
    }
    
    /// Pull word from stack
    #[inline]
    fn pull_word(&mut self, memory: &Memory) -> u16 {
        let lo = self.pull_byte(memory) as u16;
        let hi = self.pull_byte(memory) as u16;
        lo | (hi << 8)
    }
    
    /// Execute an opcode and return cycles taken
    fn execute_opcode(&mut self, opcode: u8, memory: &mut Memory) -> u8 {
        match opcode {
            // LDA - Load Accumulator
            0xA9 => self.op_lda_immediate(memory),
            0xA5 => self.op_lda_direct_page(memory),
            0xB5 => self.op_lda_direct_page_x(memory),
            0xAD => self.op_lda_absolute(memory),
            0xBD => self.op_lda_absolute_x(memory),
            0xB9 => self.op_lda_absolute_y(memory),
            
            // LDX - Load X Register
            0xA2 => self.op_ldx_immediate(memory),
            0xA6 => self.op_ldx_direct_page(memory),
            0xB6 => self.op_ldx_direct_page_y(memory),
            0xAE => self.op_ldx_absolute(memory),
            0xBE => self.op_ldx_absolute_y(memory),
            
            // LDY - Load Y Register
            0xA0 => self.op_ldy_immediate(memory),
            0xA4 => self.op_ldy_direct_page(memory),
            0xB4 => self.op_ldy_direct_page_x(memory),
            0xAC => self.op_ldy_absolute(memory),
            0xBC => self.op_ldy_absolute_x(memory),
            
            // STA - Store Accumulator
            0x85 => self.op_sta_direct_page(memory),
            0x95 => self.op_sta_direct_page_x(memory),
            0x8D => self.op_sta_absolute(memory),
            0x9D => self.op_sta_absolute_x(memory),
            0x99 => self.op_sta_absolute_y(memory),
            
            // STX - Store X Register
            0x86 => self.op_stx_direct_page(memory),
            0x96 => self.op_stx_direct_page_y(memory),
            0x8E => self.op_stx_absolute(memory),
            
            // STY - Store Y Register
            0x84 => self.op_sty_direct_page(memory),
            0x94 => self.op_sty_direct_page_x(memory),
            0x8C => self.op_sty_absolute(memory),
            
            // STZ - Store Zero
            0x64 => self.op_stz_direct_page(memory),
            0x74 => self.op_stz_direct_page_x(memory),
            0x9C => self.op_stz_absolute(memory),
            0x9E => self.op_stz_absolute_x(memory),
            
            // Transfer Instructions
            0xAA => self.op_tax(memory),
            0xA8 => self.op_tay(memory),
            0xBA => self.op_tsx(memory),
            0x8A => self.op_txa(memory),
            0x9A => self.op_txs(memory),
            0x98 => self.op_tya(memory),
            
            // Stack Operations
            0x48 => self.op_pha(memory),
            0x68 => self.op_pla(memory),
            0x08 => self.op_php(memory),
            0x28 => self.op_plp(memory),
            0xDA => self.op_phx(memory),
            0xFA => self.op_plx(memory),
            0x5A => self.op_phy(memory),
            0x7A => self.op_ply(memory),
            
            // Branches
            0x90 => self.op_bcc(memory),
            0xB0 => self.op_bcs(memory),
            0xF0 => self.op_beq(memory),
            0x30 => self.op_bmi(memory),
            0xD0 => self.op_bne(memory),
            0x10 => self.op_bpl(memory),
            0x50 => self.op_bvc(memory),
            0x70 => self.op_bvs(memory),
            0x80 => self.op_bra(memory),
            
            // Jumps
            0x4C => self.op_jmp_absolute(memory),
            0x20 => self.op_jsr_absolute(memory),
            0x60 => self.op_rts(memory),
            
            // Arithmetic - ADC
            0x69 => self.op_adc_immediate(memory),
            0x65 => self.op_adc_direct_page(memory),
            0x75 => self.op_adc_direct_page_x(memory),
            0x6D => self.op_adc_absolute(memory),
            0x7D => self.op_adc_absolute_x(memory),
            0x79 => self.op_adc_absolute_y(memory),
            
            // Arithmetic - SBC
            0xE9 => self.op_sbc_immediate(memory),
            0xE5 => self.op_sbc_direct_page(memory),
            0xF5 => self.op_sbc_direct_page_x(memory),
            0xED => self.op_sbc_absolute(memory),
            0xFD => self.op_sbc_absolute_x(memory),
            0xF9 => self.op_sbc_absolute_y(memory),
            
            // Logical - AND
            0x29 => self.op_and_immediate(memory),
            0x25 => self.op_and_direct_page(memory),
            0x35 => self.op_and_direct_page_x(memory),
            0x2D => self.op_and_absolute(memory),
            0x3D => self.op_and_absolute_x(memory),
            0x39 => self.op_and_absolute_y(memory),
            
            // Logical - ORA
            0x09 => self.op_ora_immediate(memory),
            0x05 => self.op_ora_direct_page(memory),
            0x15 => self.op_ora_direct_page_x(memory),
            0x0D => self.op_ora_absolute(memory),
            0x1D => self.op_ora_absolute_x(memory),
            0x19 => self.op_ora_absolute_y(memory),
            
            // Logical - EOR
            0x49 => self.op_eor_immediate(memory),
            0x45 => self.op_eor_direct_page(memory),
            0x55 => self.op_eor_direct_page_x(memory),
            0x4D => self.op_eor_absolute(memory),
            0x5D => self.op_eor_absolute_x(memory),
            0x59 => self.op_eor_absolute_y(memory),
            
            // Comparisons
            0xC9 => self.op_cmp_immediate(memory),
            0xC5 => self.op_cmp_direct_page(memory),
            0xD5 => self.op_cmp_direct_page_x(memory),
            0xCD => self.op_cmp_absolute(memory),
            0xDD => self.op_cmp_absolute_x(memory),
            0xD9 => self.op_cmp_absolute_y(memory),
            
            0xE0 => self.op_cpx_immediate(memory),
            0xE4 => self.op_cpx_direct_page(memory),
            0xEC => self.op_cpx_absolute(memory),
            
            0xC0 => self.op_cpy_immediate(memory),
            0xC4 => self.op_cpy_direct_page(memory),
            0xCC => self.op_cpy_absolute(memory),
            
            // Bit Operations
            0x89 => self.op_bit_immediate(memory),
            0x24 => self.op_bit_direct_page(memory),
            0x34 => self.op_bit_direct_page_x(memory),
            0x2C => self.op_bit_absolute(memory),
            0x3C => self.op_bit_absolute_x(memory),
            
            // Shifts and Rotates
            0x0A => self.op_asl_accumulator(memory),
            0x06 => self.op_asl_direct_page(memory),
            0x16 => self.op_asl_direct_page_x(memory),
            0x0E => self.op_asl_absolute(memory),
            0x1E => self.op_asl_absolute_x(memory),
            
            0x4A => self.op_lsr_accumulator(memory),
            0x46 => self.op_lsr_direct_page(memory),
            0x56 => self.op_lsr_direct_page_x(memory),
            0x4E => self.op_lsr_absolute(memory),
            0x5E => self.op_lsr_absolute_x(memory),
            
            0x2A => self.op_rol_accumulator(memory),
            0x26 => self.op_rol_direct_page(memory),
            0x36 => self.op_rol_direct_page_x(memory),
            0x2E => self.op_rol_absolute(memory),
            0x3E => self.op_rol_absolute_x(memory),
            
            0x6A => self.op_ror_accumulator(memory),
            0x66 => self.op_ror_direct_page(memory),
            0x76 => self.op_ror_direct_page_x(memory),
            0x6E => self.op_ror_absolute(memory),
            0x7E => self.op_ror_absolute_x(memory),
            
            // Increment/Decrement
            0xE8 => self.op_inx(memory),
            0xC8 => self.op_iny(memory),
            0xCA => self.op_dex(memory),
            0x88 => self.op_dey(memory),
            
            0xE6 => self.op_inc_direct_page(memory),
            0xF6 => self.op_inc_direct_page_x(memory),
            0xEE => self.op_inc_absolute(memory),
            0xFE => self.op_inc_absolute_x(memory),
            0x1A => self.op_inc_accumulator(memory),
            
            0xC6 => self.op_dec_direct_page(memory),
            0xD6 => self.op_dec_direct_page_x(memory),
            0xCE => self.op_dec_absolute(memory),
            0xDE => self.op_dec_absolute_x(memory),
            0x3A => self.op_dec_accumulator(memory),
            
            // Flag Operations
            0x18 => self.op_clc(memory),
            0xD8 => self.op_cld(memory),
            0x58 => self.op_cli(memory),
            0xB8 => self.op_clv(memory),
            0x38 => self.op_sec(memory),
            0xF8 => self.op_sed(memory),
            0x78 => self.op_sei(memory),
            
            // System
            0xEA => self.op_nop(memory),
            
            _ => {
                // Unknown opcode - treat as NOP for now
                2
            }
        }
    }
    
    // Addressing mode helpers
    
    #[inline]
    fn addr_direct_page(&mut self, memory: &Memory) -> u32 {
        let offset = self.fetch_byte(memory) as u16;
        let addr = self.d.wrapping_add(offset);
        addr as u32
    }
    
    #[inline]
    fn addr_direct_page_x(&mut self, memory: &Memory) -> u32 {
        let offset = self.fetch_byte(memory) as u16;
        let addr = self.d.wrapping_add(offset).wrapping_add(self.x);
        addr as u32
    }
    
    #[inline]
    fn addr_direct_page_y(&mut self, memory: &Memory) -> u32 {
        let offset = self.fetch_byte(memory) as u16;
        let addr = self.d.wrapping_add(offset).wrapping_add(self.y);
        addr as u32
    }
    
    #[inline]
    fn addr_absolute(&mut self, memory: &Memory) -> u32 {
        let addr = self.fetch_word(memory);
        ((self.dbr as u32) << 16) | (addr as u32)
    }
    
    #[inline]
    fn addr_absolute_x(&mut self, memory: &Memory) -> u32 {
        let addr = self.fetch_word(memory).wrapping_add(self.x);
        ((self.dbr as u32) << 16) | (addr as u32)
    }
    
    #[inline]
    fn addr_absolute_y(&mut self, memory: &Memory) -> u32 {
        let addr = self.fetch_word(memory).wrapping_add(self.y);
        ((self.dbr as u32) << 16) | (addr as u32)
    }
    
    // ===== INSTRUCTION IMPLEMENTATIONS - PHASE 1 =====
    
    // LDA - Load Accumulator
    
    #[inline]
    fn op_lda_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            // 8-bit mode
            let value = self.fetch_byte(memory);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            2
        } else {
            // 16-bit mode
            let value = self.fetch_word(memory);
            self.a = value;
            self.update_nz_16(value);
            3
        }
    }
    
    #[inline]
    fn op_lda_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            3
        } else {
            let value = memory.read_word(addr);
            self.a = value;
            self.update_nz_16(value);
            4
        }
    }
    
    #[inline]
    fn op_lda_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.a = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_lda_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.a = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_lda_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.a = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_lda_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.a = value;
            self.update_nz_16(value);
            5
        }
    }
    
    // LDX - Load X Register
    
    #[inline]
    fn op_ldx_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.fetch_byte(memory);
            self.x = value as u16;
            self.update_nz_8(value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.x = value;
            self.update_nz_16(value);
            3
        }
    }
    
    #[inline]
    fn op_ldx_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.x = value as u16;
            self.update_nz_8(value);
            3
        } else {
            let value = memory.read_word(addr);
            self.x = value;
            self.update_nz_16(value);
            4
        }
    }
    
    #[inline]
    fn op_ldx_direct_page_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_y(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.x = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.x = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_ldx_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.x = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.x = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_ldx_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.x = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.x = value;
            self.update_nz_16(value);
            5
        }
    }
    
    // LDY - Load Y Register
    
    #[inline]
    fn op_ldy_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.fetch_byte(memory);
            self.y = value as u16;
            self.update_nz_8(value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.y = value;
            self.update_nz_16(value);
            3
        }
    }
    
    #[inline]
    fn op_ldy_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.y = value as u16;
            self.update_nz_8(value);
            3
        } else {
            let value = memory.read_word(addr);
            self.y = value;
            self.update_nz_16(value);
            4
        }
    }
    
    #[inline]
    fn op_ldy_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.y = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.y = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_ldy_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.y = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.y = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_ldy_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.y = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.y = value;
            self.update_nz_16(value);
            5
        }
    }
    
    // STA - Store Accumulator
    
    #[inline]
    fn op_sta_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            memory.write(addr, (self.a & 0xFF) as u8);
            3
        } else {
            memory.write_word(addr, self.a);
            4
        }
    }
    
    #[inline]
    fn op_sta_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            memory.write(addr, (self.a & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.a);
            5
        }
    }
    
    #[inline]
    fn op_sta_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            memory.write(addr, (self.a & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.a);
            5
        }
    }
    
    #[inline]
    fn op_sta_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            memory.write(addr, (self.a & 0xFF) as u8);
            5
        } else {
            memory.write_word(addr, self.a);
            6
        }
    }
    
    #[inline]
    fn op_sta_absolute_y(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            memory.write(addr, (self.a & 0xFF) as u8);
            5
        } else {
            memory.write_word(addr, self.a);
            6
        }
    }
    
    // STX - Store X Register
    
    #[inline]
    fn op_stx_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            memory.write(addr, (self.x & 0xFF) as u8);
            3
        } else {
            memory.write_word(addr, self.x);
            4
        }
    }
    
    #[inline]
    fn op_stx_direct_page_y(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_y(memory);
        if self.p.x {
            memory.write(addr, (self.x & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.x);
            5
        }
    }
    
    #[inline]
    fn op_stx_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            memory.write(addr, (self.x & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.x);
            5
        }
    }
    
    // STY - Store Y Register
    
    #[inline]
    fn op_sty_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            memory.write(addr, (self.y & 0xFF) as u8);
            3
        } else {
            memory.write_word(addr, self.y);
            4
        }
    }
    
    #[inline]
    fn op_sty_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.x {
            memory.write(addr, (self.y & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.y);
            5
        }
    }
    
    #[inline]
    fn op_sty_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            memory.write(addr, (self.y & 0xFF) as u8);
            4
        } else {
            memory.write_word(addr, self.y);
            5
        }
    }
    
    // STZ - Store Zero
    
    #[inline]
    fn op_stz_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            memory.write(addr, 0);
            3
        } else {
            memory.write_word(addr, 0);
            4
        }
    }
    
    #[inline]
    fn op_stz_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            memory.write(addr, 0);
            4
        } else {
            memory.write_word(addr, 0);
            5
        }
    }
    
    #[inline]
    fn op_stz_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            memory.write(addr, 0);
            4
        } else {
            memory.write_word(addr, 0);
            5
        }
    }
    
    #[inline]
    fn op_stz_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            memory.write(addr, 0);
            5
        } else {
            memory.write_word(addr, 0);
            6
        }
    }
    
    // Transfer Instructions
    
    #[inline]
    fn op_tax(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            self.x = self.a & 0xFF;
            self.update_nz_8(self.x as u8);
        } else {
            self.x = self.a;
            self.update_nz_16(self.x);
        }
        2
    }
    
    #[inline]
    fn op_tay(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            self.y = self.a & 0xFF;
            self.update_nz_8(self.y as u8);
        } else {
            self.y = self.a;
            self.update_nz_16(self.y);
        }
        2
    }
    
    #[inline]
    fn op_tsx(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            self.x = self.s & 0xFF;
            self.update_nz_8(self.x as u8);
        } else {
            self.x = self.s;
            self.update_nz_16(self.x);
        }
        2
    }
    
    #[inline]
    fn op_txa(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            self.a = (self.a & 0xFF00) | (self.x & 0xFF);
            self.update_nz_8(self.a as u8);
        } else {
            self.a = self.x;
            self.update_nz_16(self.a);
        }
        2
    }
    
    #[inline]
    fn op_txs(&mut self, _memory: &Memory) -> u8 {
        self.s = if self.p.e {
            0x0100 | (self.x & 0xFF)
        } else {
            self.x
        };
        2
    }
    
    #[inline]
    fn op_tya(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            self.a = (self.a & 0xFF00) | (self.y & 0xFF);
            self.update_nz_8(self.a as u8);
        } else {
            self.a = self.y;
            self.update_nz_16(self.a);
        }
        2
    }
    
    // Stack Operations
    
    #[inline]
    fn op_pha(&mut self, memory: &mut Memory) -> u8 {
        if self.p.m {
            self.push_byte(memory, (self.a & 0xFF) as u8);
            3
        } else {
            self.push_word(memory, self.a);
            4
        }
    }
    
    #[inline]
    fn op_pla(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.pull_byte(memory);
            self.a = (self.a & 0xFF00) | (value as u16);
            self.update_nz_8(value);
            4
        } else {
            let value = self.pull_word(memory);
            self.a = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_php(&mut self, memory: &mut Memory) -> u8 {
        self.push_byte(memory, self.p.to_byte());
        3
    }
    
    #[inline]
    fn op_plp(&mut self, memory: &Memory) -> u8 {
        let value = self.pull_byte(memory);
        self.p.from_byte(value);
        4
    }
    
    #[inline]
    fn op_phx(&mut self, memory: &mut Memory) -> u8 {
        if self.p.x {
            self.push_byte(memory, (self.x & 0xFF) as u8);
            3
        } else {
            self.push_word(memory, self.x);
            4
        }
    }
    
    #[inline]
    fn op_plx(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.pull_byte(memory);
            self.x = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = self.pull_word(memory);
            self.x = value;
            self.update_nz_16(value);
            5
        }
    }
    
    #[inline]
    fn op_phy(&mut self, memory: &mut Memory) -> u8 {
        if self.p.x {
            self.push_byte(memory, (self.y & 0xFF) as u8);
            3
        } else {
            self.push_word(memory, self.y);
            4
        }
    }
    
    #[inline]
    fn op_ply(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.pull_byte(memory);
            self.y = value as u16;
            self.update_nz_8(value);
            4
        } else {
            let value = self.pull_word(memory);
            self.y = value;
            self.update_nz_16(value);
            5
        }
    }
    
    // Branch Instructions
    
    #[inline]
    fn op_bcc(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if !self.p.c {
            self.pc = self.pc.wrapping_add(offset as u16);
            3 // Branch taken
        } else {
            2 // Branch not taken
        }
    }
    
    #[inline]
    fn op_bcs(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if self.p.c {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_beq(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if self.p.z {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bmi(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if self.p.n {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bne(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if !self.p.z {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bpl(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if !self.p.n {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bvc(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if !self.p.v {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bvs(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        if self.p.v {
            self.pc = self.pc.wrapping_add(offset as u16);
            3
        } else {
            2
        }
    }
    
    #[inline]
    fn op_bra(&mut self, memory: &Memory) -> u8 {
        let offset = self.fetch_byte(memory) as i8;
        self.pc = self.pc.wrapping_add(offset as u16);
        3
    }
    
    // Jump Instructions
    
    #[inline]
    fn op_jmp_absolute(&mut self, memory: &Memory) -> u8 {
        self.pc = self.fetch_word(memory);
        3
    }
    
    #[inline]
    fn op_jsr_absolute(&mut self, memory: &mut Memory) -> u8 {
        let target = self.fetch_word(memory);
        let return_addr = self.pc.wrapping_sub(1);
        self.push_word(memory, return_addr);
        self.pc = target;
        6
    }
    
    #[inline]
    fn op_rts(&mut self, memory: &Memory) -> u8 {
        let addr = self.pull_word(memory);
        self.pc = addr.wrapping_add(1);
        6
    }
    
    // Flag Operations
    
    #[inline]
    fn op_clc(&mut self, _memory: &Memory) -> u8 {
        self.p.c = false;
        2
    }
    
    #[inline]
    fn op_cld(&mut self, _memory: &Memory) -> u8 {
        self.p.d = false;
        2
    }
    
    #[inline]
    fn op_cli(&mut self, _memory: &Memory) -> u8 {
        self.p.i = false;
        2
    }
    
    #[inline]
    fn op_clv(&mut self, _memory: &Memory) -> u8 {
        self.p.v = false;
        2
    }
    
    #[inline]
    fn op_sec(&mut self, _memory: &Memory) -> u8 {
        self.p.c = true;
        2
    }
    
    #[inline]
    fn op_sed(&mut self, _memory: &Memory) -> u8 {
        self.p.d = true;
        2
    }
    
    #[inline]
    fn op_sei(&mut self, _memory: &Memory) -> u8 {
        self.p.i = true;
        2
    }
    
    // System
    
    #[inline]
    fn op_nop(&mut self, _memory: &Memory) -> u8 {
        2
    }
    
    // ===== ARITHMETIC OPERATIONS =====
    
    // ADC - Add with Carry
    
    #[inline]
    fn op_adc_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            self.adc_8(value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.adc_16(value);
            3
        }
    }
    
    #[inline]
    fn op_adc_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.adc_8(value);
            3
        } else {
            let value = memory.read_word(addr);
            self.adc_16(value);
            4
        }
    }
    
    #[inline]
    fn op_adc_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.adc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.adc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_adc_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.adc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.adc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_adc_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.adc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.adc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_adc_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.adc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.adc_16(value);
            5
        }
    }
    
    #[inline]
    fn adc_8(&mut self, value: u8) {
        let a = (self.a & 0xFF) as u8;
        let carry = if self.p.c { 1 } else { 0 };
        
        if self.p.d {
            // Decimal mode
            let mut al = (a & 0x0F) + (value & 0x0F) + carry;
            if al > 0x09 {
                al += 0x06;
            }
            let mut ah = (a >> 4) + (value >> 4) + if al > 0x0F { 1 } else { 0 };
            
            self.p.v = ((!(a ^ value)) & (a ^ (ah << 4))) & 0x80 != 0;
            
            if ah > 0x09 {
                ah += 0x06;
            }
            
            self.p.c = ah > 0x0F;
            let result = ((ah & 0x0F) << 4) | (al & 0x0F);
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            // Binary mode
            let result = a as u16 + value as u16 + carry as u16;
            self.p.c = result > 0xFF;
            self.p.v = ((!(a ^ value)) & (a ^ (result as u8))) & 0x80 != 0;
            let result = result as u8;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        }
    }
    
    #[inline]
    fn adc_16(&mut self, value: u16) {
        let a = self.a;
        let carry = if self.p.c { 1 } else { 0 };
        
        if self.p.d {
            // Decimal mode for 16-bit
            let mut result = 0u16;
            let mut c = carry;
            
            for i in 0..4 {
                let shift = i * 4;
                let mut digit = ((a >> shift) & 0x0F) + ((value >> shift) & 0x0F) + c;
                if digit > 0x09 {
                    digit += 0x06;
                }
                c = if digit > 0x0F { 1 } else { 0 };
                result |= (digit & 0x0F) << shift;
            }
            
            self.p.c = c != 0;
            self.p.v = ((!(a ^ value)) & (a ^ result)) & 0x8000 != 0;
            self.a = result;
            self.update_nz_16(result);
        } else {
            // Binary mode
            let result = a as u32 + value as u32 + carry as u32;
            self.p.c = result > 0xFFFF;
            self.p.v = ((!(a ^ value)) & (a ^ (result as u16))) & 0x8000 != 0;
            self.a = result as u16;
            self.update_nz_16(self.a);
        }
    }
    
    // SBC - Subtract with Carry
    
    #[inline]
    fn op_sbc_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            self.sbc_8(value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.sbc_16(value);
            3
        }
    }
    
    #[inline]
    fn op_sbc_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.sbc_8(value);
            3
        } else {
            let value = memory.read_word(addr);
            self.sbc_16(value);
            4
        }
    }
    
    #[inline]
    fn op_sbc_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.sbc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.sbc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_sbc_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.sbc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.sbc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_sbc_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.sbc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.sbc_16(value);
            5
        }
    }
    
    #[inline]
    fn op_sbc_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.sbc_8(value);
            4
        } else {
            let value = memory.read_word(addr);
            self.sbc_16(value);
            5
        }
    }
    
    #[inline]
    fn sbc_8(&mut self, value: u8) {
        let a = (self.a & 0xFF) as u8;
        let borrow = if self.p.c { 0 } else { 1 };
        
        if self.p.d {
            // Decimal mode
            let mut al = (a & 0x0F) as i16 - (value & 0x0F) as i16 - borrow as i16;
            if al < 0 {
                al -= 0x06;
            }
            let mut ah = (a >> 4) as i16 - (value >> 4) as i16 - if al < 0 { 1 } else { 0 };
            
            self.p.v = ((a ^ value) & (a ^ ((ah << 4) as u8))) & 0x80 != 0;
            
            if ah < 0 {
                ah -= 0x06;
            }
            
            self.p.c = ah >= 0;
            let result = (((ah & 0x0F) << 4) | (al & 0x0F)) as u8;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            // Binary mode
            let result = a as i16 - value as i16 - borrow as i16;
            self.p.c = result >= 0;
            self.p.v = ((a ^ value) & (a ^ (result as u8))) & 0x80 != 0;
            let result = result as u8;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        }
    }
    
    #[inline]
    fn sbc_16(&mut self, value: u16) {
        let a = self.a;
        let borrow = if self.p.c { 0 } else { 1 };
        
        if self.p.d {
            // Decimal mode for 16-bit
            let mut result = 0u16;
            let mut b = borrow;
            
            for i in 0..4 {
                let shift = i * 4;
                let mut digit = ((a >> shift) & 0x0F) as i16 - ((value >> shift) & 0x0F) as i16 - b as i16;
                if digit < 0 {
                    digit -= 0x06;
                }
                b = if digit < 0 { 1 } else { 0 };
                result |= ((digit & 0x0F) as u16) << shift;
            }
            
            self.p.c = b == 0;
            self.p.v = ((a ^ value) & (a ^ result)) & 0x8000 != 0;
            self.a = result;
            self.update_nz_16(result);
        } else {
            // Binary mode
            let result = a as i32 - value as i32 - borrow as i32;
            self.p.c = result >= 0;
            self.p.v = ((a ^ value) & (a ^ (result as u16))) & 0x8000 != 0;
            self.a = result as u16;
            self.update_nz_16(self.a);
        }
    }
    
    // ===== LOGICAL OPERATIONS =====
    
    // AND - Logical AND
    
    #[inline]
    fn op_and_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            let value = self.fetch_word(memory);
            self.a &= value;
            self.update_nz_16(self.a);
            3
        }
    }
    
    #[inline]
    fn op_and_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            3
        } else {
            let value = memory.read_word(addr);
            self.a &= value;
            self.update_nz_16(self.a);
            4
        }
    }
    
    #[inline]
    fn op_and_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a &= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_and_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a &= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_and_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a &= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_and_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a &= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    // ORA - Logical OR
    
    #[inline]
    fn op_ora_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            let value = self.fetch_word(memory);
            self.a |= value;
            self.update_nz_16(self.a);
            3
        }
    }
    
    #[inline]
    fn op_ora_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            3
        } else {
            let value = memory.read_word(addr);
            self.a |= value;
            self.update_nz_16(self.a);
            4
        }
    }
    
    #[inline]
    fn op_ora_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a |= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_ora_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a |= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_ora_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a |= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_ora_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 | value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a |= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    // EOR - Logical Exclusive OR
    
    #[inline]
    fn op_eor_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            let value = self.fetch_word(memory);
            self.a ^= value;
            self.update_nz_16(self.a);
            3
        }
    }
    
    #[inline]
    fn op_eor_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            3
        } else {
            let value = memory.read_word(addr);
            self.a ^= value;
            self.update_nz_16(self.a);
            4
        }
    }
    
    #[inline]
    fn op_eor_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a ^= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_eor_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a ^= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_eor_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a ^= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    #[inline]
    fn op_eor_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 ^ value;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            4
        } else {
            let value = memory.read_word(addr);
            self.a ^= value;
            self.update_nz_16(self.a);
            5
        }
    }
    
    // ===== COMPARISON OPERATIONS =====
    
    // CMP - Compare Accumulator
    
    #[inline]
    fn op_cmp_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            self.compare_8((self.a & 0xFF) as u8, value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.compare_16(self.a, value);
            3
        }
    }
    
    #[inline]
    fn op_cmp_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.compare_8((self.a & 0xFF) as u8, value);
            3
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.a, value);
            4
        }
    }
    
    #[inline]
    fn op_cmp_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.compare_8((self.a & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.a, value);
            5
        }
    }
    
    #[inline]
    fn op_cmp_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.compare_8((self.a & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.a, value);
            5
        }
    }
    
    #[inline]
    fn op_cmp_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.compare_8((self.a & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.a, value);
            5
        }
    }
    
    #[inline]
    fn op_cmp_absolute_y(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_y(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.compare_8((self.a & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.a, value);
            5
        }
    }
    
    // CPX - Compare X Register
    
    #[inline]
    fn op_cpx_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.fetch_byte(memory);
            self.compare_8((self.x & 0xFF) as u8, value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.compare_16(self.x, value);
            3
        }
    }
    
    #[inline]
    fn op_cpx_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.compare_8((self.x & 0xFF) as u8, value);
            3
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.x, value);
            4
        }
    }
    
    #[inline]
    fn op_cpx_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.compare_8((self.x & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.x, value);
            5
        }
    }
    
    // CPY - Compare Y Register
    
    #[inline]
    fn op_cpy_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.x {
            let value = self.fetch_byte(memory);
            self.compare_8((self.y & 0xFF) as u8, value);
            2
        } else {
            let value = self.fetch_word(memory);
            self.compare_16(self.y, value);
            3
        }
    }
    
    #[inline]
    fn op_cpy_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.compare_8((self.y & 0xFF) as u8, value);
            3
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.y, value);
            4
        }
    }
    
    #[inline]
    fn op_cpy_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.x {
            let value = memory.read(addr);
            self.compare_8((self.y & 0xFF) as u8, value);
            4
        } else {
            let value = memory.read_word(addr);
            self.compare_16(self.y, value);
            5
        }
    }
    
    #[inline]
    fn compare_8(&mut self, reg: u8, value: u8) {
        let result = reg.wrapping_sub(value);
        self.p.c = reg >= value;
        self.p.z = result == 0;
        self.p.n = result & 0x80 != 0;
    }
    
    #[inline]
    fn compare_16(&mut self, reg: u16, value: u16) {
        let result = reg.wrapping_sub(value);
        self.p.c = reg >= value;
        self.p.z = result == 0;
        self.p.n = result & 0x8000 != 0;
    }
    
    // BIT - Bit Test
    
    #[inline]
    fn op_bit_immediate(&mut self, memory: &Memory) -> u8 {
        if self.p.m {
            let value = self.fetch_byte(memory);
            let result = (self.a & 0xFF) as u8 & value;
            self.p.z = result == 0;
            // Immediate mode does NOT affect N and V flags
            2
        } else {
            let value = self.fetch_word(memory);
            let result = self.a & value;
            self.p.z = result == 0;
            // Immediate mode does NOT affect N and V flags
            3
        }
    }
    
    #[inline]
    fn op_bit_direct_page(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.p.z = result == 0;
            self.p.n = value & 0x80 != 0;
            self.p.v = value & 0x40 != 0;
            3
        } else {
            let value = memory.read_word(addr);
            let result = self.a & value;
            self.p.z = result == 0;
            self.p.n = value & 0x8000 != 0;
            self.p.v = value & 0x4000 != 0;
            4
        }
    }
    
    #[inline]
    fn op_bit_direct_page_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.p.z = result == 0;
            self.p.n = value & 0x80 != 0;
            self.p.v = value & 0x40 != 0;
            4
        } else {
            let value = memory.read_word(addr);
            let result = self.a & value;
            self.p.z = result == 0;
            self.p.n = value & 0x8000 != 0;
            self.p.v = value & 0x4000 != 0;
            5
        }
    }
    
    #[inline]
    fn op_bit_absolute(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.p.z = result == 0;
            self.p.n = value & 0x80 != 0;
            self.p.v = value & 0x40 != 0;
            4
        } else {
            let value = memory.read_word(addr);
            let result = self.a & value;
            self.p.z = result == 0;
            self.p.n = value & 0x8000 != 0;
            self.p.v = value & 0x4000 != 0;
            5
        }
    }
    
    #[inline]
    fn op_bit_absolute_x(&mut self, memory: &Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = (self.a & 0xFF) as u8 & value;
            self.p.z = result == 0;
            self.p.n = value & 0x80 != 0;
            self.p.v = value & 0x40 != 0;
            4
        } else {
            let value = memory.read_word(addr);
            let result = self.a & value;
            self.p.z = result == 0;
            self.p.n = value & 0x8000 != 0;
            self.p.v = value & 0x4000 != 0;
            5
        }
    }
    
    // ===== SHIFT AND ROTATE OPERATIONS =====
    
    // ASL - Arithmetic Shift Left
    
    #[inline]
    fn op_asl_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let value = (self.a & 0xFF) as u8;
            self.p.c = value & 0x80 != 0;
            let result = value << 1;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            self.p.c = self.a & 0x8000 != 0;
            self.a <<= 1;
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_asl_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x80 != 0;
            let result = value << 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x8000 != 0;
            let result = value << 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_asl_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x80 != 0;
            let result = value << 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x8000 != 0;
            let result = value << 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_asl_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x80 != 0;
            let result = value << 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x8000 != 0;
            let result = value << 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_asl_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x80 != 0;
            let result = value << 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x8000 != 0;
            let result = value << 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // LSR - Logical Shift Right
    
    #[inline]
    fn op_lsr_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let value = (self.a & 0xFF) as u8;
            self.p.c = value & 0x01 != 0;
            let result = value >> 1;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            self.p.c = self.a & 0x0001 != 0;
            self.a >>= 1;
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_lsr_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x01 != 0;
            let result = value >> 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x0001 != 0;
            let result = value >> 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_lsr_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x01 != 0;
            let result = value >> 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x0001 != 0;
            let result = value >> 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_lsr_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x01 != 0;
            let result = value >> 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x0001 != 0;
            let result = value >> 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_lsr_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            self.p.c = value & 0x01 != 0;
            let result = value >> 1;
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            self.p.c = value & 0x0001 != 0;
            let result = value >> 1;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // ROL - Rotate Left
    
    #[inline]
    fn op_rol_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let value = (self.a & 0xFF) as u8;
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x80 != 0;
            let result = (value << 1) | old_carry;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = self.a & 0x8000 != 0;
            self.a = (self.a << 1) | old_carry;
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_rol_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x80 != 0;
            let result = (value << 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x8000 != 0;
            let result = (value << 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_rol_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x80 != 0;
            let result = (value << 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x8000 != 0;
            let result = (value << 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_rol_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x80 != 0;
            let result = (value << 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x8000 != 0;
            let result = (value << 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_rol_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x80 != 0;
            let result = (value << 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 1 } else { 0 };
            self.p.c = value & 0x8000 != 0;
            let result = (value << 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // ROR - Rotate Right
    
    #[inline]
    fn op_ror_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let value = (self.a & 0xFF) as u8;
            let old_carry = if self.p.c { 0x80 } else { 0 };
            self.p.c = value & 0x01 != 0;
            let result = (value >> 1) | old_carry;
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            let old_carry = if self.p.c { 0x8000 } else { 0 };
            self.p.c = self.a & 0x0001 != 0;
            self.a = (self.a >> 1) | old_carry;
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_ror_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 0x80 } else { 0 };
            self.p.c = value & 0x01 != 0;
            let result = (value >> 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 0x8000 } else { 0 };
            self.p.c = value & 0x0001 != 0;
            let result = (value >> 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_ror_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 0x80 } else { 0 };
            self.p.c = value & 0x01 != 0;
            let result = (value >> 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 0x8000 } else { 0 };
            self.p.c = value & 0x0001 != 0;
            let result = (value >> 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_ror_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 0x80 } else { 0 };
            self.p.c = value & 0x01 != 0;
            let result = (value >> 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 0x8000 } else { 0 };
            self.p.c = value & 0x0001 != 0;
            let result = (value >> 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_ror_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let old_carry = if self.p.c { 0x80 } else { 0 };
            self.p.c = value & 0x01 != 0;
            let result = (value >> 1) | old_carry;
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            let old_carry = if self.p.c { 0x8000 } else { 0 };
            self.p.c = value & 0x0001 != 0;
            let result = (value >> 1) | old_carry;
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // ===== INCREMENT AND DECREMENT OPERATIONS =====
    
    // INC - Increment Memory
    
    #[inline]
    fn op_inc_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let result = ((self.a & 0xFF) as u8).wrapping_add(1);
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            self.a = self.a.wrapping_add(1);
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_inc_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_add(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_add(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_inc_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_add(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_add(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_inc_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_add(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_add(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_inc_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_add(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_add(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // DEC - Decrement Memory
    
    #[inline]
    fn op_dec_accumulator(&mut self, _memory: &Memory) -> u8 {
        if self.p.m {
            let result = ((self.a & 0xFF) as u8).wrapping_sub(1);
            self.a = (self.a & 0xFF00) | (result as u16);
            self.update_nz_8(result);
            2
        } else {
            self.a = self.a.wrapping_sub(1);
            self.update_nz_16(self.a);
            2
        }
    }
    
    #[inline]
    fn op_dec_direct_page(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_sub(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            5
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_sub(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            6
        }
    }
    
    #[inline]
    fn op_dec_direct_page_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_direct_page_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_sub(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_sub(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_dec_absolute(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_sub(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            6
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_sub(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            7
        }
    }
    
    #[inline]
    fn op_dec_absolute_x(&mut self, memory: &mut Memory) -> u8 {
        let addr = self.addr_absolute_x(memory);
        if self.p.m {
            let value = memory.read(addr);
            let result = value.wrapping_sub(1);
            memory.write(addr, result);
            self.update_nz_8(result);
            7
        } else {
            let value = memory.read_word(addr);
            let result = value.wrapping_sub(1);
            memory.write_word(addr, result);
            self.update_nz_16(result);
            8
        }
    }
    
    // INX, INY, DEX, DEY - Register increment/decrement
    
    #[inline]
    fn op_inx(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            let result = ((self.x & 0xFF) as u8).wrapping_add(1);
            self.x = (self.x & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            self.x = self.x.wrapping_add(1);
            self.update_nz_16(self.x);
        }
        2
    }
    
    #[inline]
    fn op_iny(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            let result = ((self.y & 0xFF) as u8).wrapping_add(1);
            self.y = (self.y & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            self.y = self.y.wrapping_add(1);
            self.update_nz_16(self.y);
        }
        2
    }
    
    #[inline]
    fn op_dex(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            let result = ((self.x & 0xFF) as u8).wrapping_sub(1);
            self.x = (self.x & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            self.x = self.x.wrapping_sub(1);
            self.update_nz_16(self.x);
        }
        2
    }
    
    #[inline]
    fn op_dey(&mut self, _memory: &Memory) -> u8 {
        if self.p.x {
            let result = ((self.y & 0xFF) as u8).wrapping_sub(1);
            self.y = (self.y & 0xFF00) | (result as u16);
            self.update_nz_8(result);
        } else {
            self.y = self.y.wrapping_sub(1);
            self.update_nz_16(self.y);
        }
        2
    }
}

impl Default for Cpu65816 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;
    
    fn create_test_system() -> (Cpu65816, Memory) {
        let rom = create_test_rom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        let cpu = Cpu65816::new();
        (cpu, memory)
    }
    
    fn create_test_system_with_code(code: &[u8]) -> (Cpu65816, Memory) {
        let rom = create_test_rom_with_code(code);
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        let cpu = Cpu65816::new();
        (cpu, memory)
    }
    
    fn create_test_rom() -> Vec<u8> {
        create_test_rom_with_code(&[0xEA]) // NOP
    }
    
    fn create_test_rom_with_code(code: &[u8]) -> Vec<u8> {
        let mut rom = vec![0; 0x8000];
        let header_offset = 0x7FC0;
        
        let title = b"TEST ROM             ";
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        rom[header_offset + 0x15] = 0x20;
        rom[header_offset + 0x16] = 0x00;
        rom[header_offset + 0x17] = 0x08;
        rom[header_offset + 0x18] = 0x00;
        rom[header_offset + 0x19] = 0x01;
        rom[header_offset + 0x1C] = 0xFF;
        rom[header_offset + 0x1D] = 0xFF;
        rom[header_offset + 0x1E] = 0x00;
        rom[header_offset + 0x1F] = 0x00;
        
        // Place code at start of ROM
        rom[..code.len()].copy_from_slice(code);
        
        rom
    }
    
    #[test]
    fn test_cpu_creation() {
        let cpu = Cpu65816::new();
        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.x, 0);
        assert_eq!(cpu.y, 0);
        assert_eq!(cpu.s, 0x01FF);
        assert!(cpu.p.e); // Emulation mode
        assert!(cpu.p.m); // 8-bit accumulator
        assert!(cpu.p.x); // 8-bit index
    }
    
    #[test]
    fn test_cpu_reset() {
        // Create ROM with reset vector
        let mut rom = vec![0; 0x8000];
        let header_offset = 0x7FC0;
        
        let title = b"TEST ROM             ";
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        rom[header_offset + 0x15] = 0x20;
        rom[header_offset + 0x16] = 0x00;
        rom[header_offset + 0x17] = 0x08;
        rom[header_offset + 0x18] = 0x00;
        rom[header_offset + 0x19] = 0x01;
        rom[header_offset + 0x1C] = 0xFF;
        rom[header_offset + 0x1D] = 0xFF;
        rom[header_offset + 0x1E] = 0x00;
        rom[header_offset + 0x1F] = 0x00;
        
        // Set reset vector at $FFFC-$FFFD (LoROM: at offset $7FFC in ROM)
        rom[0x7FFC] = 0x00;
        rom[0x7FFD] = 0x80; // Reset to $8000
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        let mut cpu = Cpu65816::new();
        
        cpu.reset(&memory);
        
        assert_eq!(cpu.pc, 0x8000);
        assert_eq!(cpu.pbr, 0);
        assert!(cpu.p.e);
        assert!(cpu.p.i);
    }
    
    #[test]
    fn test_status_flags_byte_conversion() {
        let mut flags = StatusFlags::new();
        flags.n = true;
        flags.z = true;
        flags.c = true;
        
        let byte = flags.to_byte();
        assert_eq!(byte & 0x80, 0x80); // N
        assert_eq!(byte & 0x02, 0x02); // Z
        assert_eq!(byte & 0x01, 0x01); // C
        
        let mut flags2 = StatusFlags::new();
        flags2.from_byte(byte);
        assert!(flags2.n);
        assert!(flags2.z);
        assert!(flags2.c);
    }
    
    #[test]
    fn test_stack_push_pull() {
        let (mut cpu, mut memory) = create_test_system();
        cpu.reset(&memory);
        
        let initial_sp = cpu.s;
        cpu.push_byte(&mut memory, 0x42);
        assert_eq!(cpu.s, initial_sp.wrapping_sub(1));
        
        let value = cpu.pull_byte(&memory);
        assert_eq!(value, 0x42);
        assert_eq!(cpu.s, initial_sp);
    }
    
    #[test]
    fn test_lda_immediate_8bit() {
        let code = vec![0xA9, 0x42]; // LDA #$42
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.pc = 0x8000;
        cpu.pbr = 0;
        cpu.p.m = true; // 8-bit mode
        
        let cycles = cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x42);
        assert_eq!(cycles, 2);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_lda_immediate_16bit() {
        let code = vec![0xA9, 0x34, 0x12]; // LDA #$1234
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.pc = 0x8000;
        cpu.pbr = 0;
        cpu.p.m = false; // 16-bit mode
        cpu.p.e = false; // Native mode for 16-bit
        
        let cycles = cpu.step(&mut memory);
        
        assert_eq!(cpu.a, 0x1234);
        assert_eq!(cycles, 3);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_lda_zero_flag() {
        let code = vec![0xA9, 0x00]; // LDA #$00
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.pc = 0x8000;
        cpu.pbr = 0;
        cpu.p.m = true;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_lda_negative_flag() {
        let code = vec![0xA9, 0x80]; // LDA #$80
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.pc = 0x8000;
        cpu.pbr = 0;
        cpu.p.m = true;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x80);
        assert!(cpu.p.n);
        assert!(!cpu.p.z);
    }
    
    #[test]
    fn test_sta_direct_page() {
        let code = vec![0x85, 0x10]; // STA $10
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.d = 0x0000;
        cpu.p.m = true;
        cpu.pbr = 0;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(memory.read(0x0010), 0x42);
    }
    
    #[test]
    fn test_tax_transfer() {
        let code = vec![0xAA]; // TAX
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.x = true; // 8-bit index
        cpu.pbr = 0;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.x, 0x42);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_branch_taken() {
        let code = vec![0xF0, 0x05]; // BEQ +5
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.p.z = true;
        cpu.pbr = 0;
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut memory);
        
        assert_eq!(cpu.pc, 0x8007); // 0x8002 + 5
        assert_eq!(cycles, 3);
    }
    
    #[test]
    fn test_branch_not_taken() {
        let code = vec![0xF0, 0x05]; // BEQ +5
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.p.z = false;
        cpu.pbr = 0;
        cpu.pc = 0x8000;
        
        let cycles = cpu.step(&mut memory);
        
        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cycles, 2);
    }
    
    #[test]
    fn test_jsr_rts() {
        let code = vec![
            0x20, 0x05, 0x80, // JSR $8005
            0xEA,             // NOP (shouldn't reach here before RTS)
            0xEA,             // NOP
            0x60,             // RTS (at $8005)
        ];
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.pc = 0x8000;
        cpu.pbr = 0;
        cpu.s = 0x01FF;
        
        cpu.step(&mut memory); // JSR
        assert_eq!(cpu.pc, 0x8005);
        
        cpu.step(&mut memory); // RTS
        assert_eq!(cpu.pc, 0x8003);
    }
    
    // ===== PHASE 2 TESTS =====
    
    #[test]
    fn test_adc_8bit_no_carry() {
        let code = vec![0x69, 0x42]; // ADC #$42
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x10;
        cpu.p.m = true;
        cpu.p.c = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x52);
        assert!(!cpu.p.c);
        assert!(!cpu.p.v);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_adc_8bit_with_carry() {
        let code = vec![0x69, 0xFF]; // ADC #$FF
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x01;
        cpu.p.m = true;
        cpu.p.c = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x01); // $01 + $FF + 1 = $101 (wraps to $01)
        assert!(cpu.p.c);
        assert!(!cpu.p.v);
    }
    
    #[test]
    fn test_adc_overflow() {
        let code = vec![0x69, 0x7F]; // ADC #$7F
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x7F;
        cpu.p.m = true;
        cpu.p.c = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0xFE);
        assert!(!cpu.p.c);
        assert!(cpu.p.v); // Overflow: positive + positive = negative
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_sbc_8bit() {
        let code = vec![0xE9, 0x10]; // SBC #$10
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x30;
        cpu.p.m = true;
        cpu.p.c = true; // No borrow
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x20);
        assert!(cpu.p.c);
        assert!(!cpu.p.v);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_sbc_borrow() {
        let code = vec![0xE9, 0x20]; // SBC #$20
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x10;
        cpu.p.m = true;
        cpu.p.c = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0xF0); // $10 - $20 = -$10 = $F0 (two's complement)
        assert!(!cpu.p.c); // Borrow occurred
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_and_immediate() {
        let code = vec![0x29, 0x0F]; // AND #$0F
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xF5;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x05);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_and_zero_result() {
        let code = vec![0x29, 0x00]; // AND #$00
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xFF;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_ora_immediate() {
        let code = vec![0x09, 0x0F]; // ORA #$0F
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xF0;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0xFF);
        assert!(!cpu.p.z);
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_eor_immediate() {
        let code = vec![0x49, 0xFF]; // EOR #$FF
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xAA;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x55);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_cmp_equal() {
        let code = vec![0xC9, 0x42]; // CMP #$42
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(cpu.p.z); // Equal
        assert!(cpu.p.c); // A >= value
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_cmp_greater() {
        let code = vec![0xC9, 0x20]; // CMP #$20
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(!cpu.p.z);
        assert!(cpu.p.c); // A >= value
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_cmp_less() {
        let code = vec![0xC9, 0x60]; // CMP #$60
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(!cpu.p.z);
        assert!(!cpu.p.c); // A < value
        assert!(cpu.p.n); // Result is negative
    }
    
    #[test]
    fn test_cpx_immediate() {
        let code = vec![0xE0, 0x42]; // CPX #$42
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.x = 0x42;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(cpu.p.z);
        assert!(cpu.p.c);
    }
    
    #[test]
    fn test_cpy_immediate() {
        let code = vec![0xC0, 0x10]; // CPY #$10
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.y = 0x20;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(!cpu.p.z);
        assert!(cpu.p.c); // Y >= value
    }
    
    #[test]
    fn test_bit_immediate() {
        let code = vec![0x89, 0x0F]; // BIT #$0F
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xF0;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert!(cpu.p.z); // (A & value) == 0
        // Immediate mode doesn't affect N and V
    }
    
    #[test]
    fn test_bit_absolute() {
        let code = vec![
            0x2C, 0x00, 0x70, // BIT $7000
        ];
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xFF;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        // Write value with N and V bits set
        memory.write(0x007000, 0xC0); // N=1, V=1
        
        cpu.step(&mut memory);
        
        assert!(!cpu.p.z); // (A & value) != 0
        assert!(cpu.p.n); // From memory bit 7
        assert!(cpu.p.v); // From memory bit 6
    }
    
    #[test]
    fn test_asl_accumulator() {
        let code = vec![0x0A]; // ASL A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x84);
        assert!(!cpu.p.c);
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_asl_carry_out() {
        let code = vec![0x0A]; // ASL A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x80;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.c); // Bit 7 shifted into carry
        assert!(cpu.p.z);
    }
    
    #[test]
    fn test_lsr_accumulator() {
        let code = vec![0x4A]; // LSR A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x21);
        assert!(!cpu.p.c);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_lsr_carry_out() {
        let code = vec![0x4A]; // LSR A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x01;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.c); // Bit 0 shifted into carry
        assert!(cpu.p.z);
    }
    
    #[test]
    fn test_rol_accumulator() {
        let code = vec![0x2A]; // ROL A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.p.c = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x85); // $42 << 1 | 1 = $85
        assert!(!cpu.p.c);
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_rol_carry() {
        let code = vec![0x2A]; // ROL A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x80;
        cpu.p.m = true;
        cpu.p.c = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.c); // Bit 7 rotated into carry
        assert!(cpu.p.z);
    }
    
    #[test]
    fn test_ror_accumulator() {
        let code = vec![0x6A]; // ROR A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.p.c = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0xA1); // $42 >> 1 | $80 = $A1
        assert!(!cpu.p.c);
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_ror_carry() {
        let code = vec![0x6A]; // ROR A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x01;
        cpu.p.m = true;
        cpu.p.c = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.c); // Bit 0 rotated into carry
        assert!(cpu.p.z);
    }
    
    #[test]
    fn test_inc_accumulator() {
        let code = vec![0x1A]; // INC A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x41;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x42);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_inc_wrap() {
        let code = vec![0x1A]; // INC A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0xFF;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x00);
        assert!(cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_dec_accumulator() {
        let code = vec![0x3A]; // DEC A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x42;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0x41);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_dec_wrap() {
        let code = vec![0x3A]; // DEC A
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x00;
        cpu.p.m = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a & 0xFF, 0xFF);
        assert!(!cpu.p.z);
        assert!(cpu.p.n);
    }
    
    #[test]
    fn test_inx() {
        let code = vec![0xE8]; // INX
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.x = 0x41;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.x & 0xFF, 0x42);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_iny() {
        let code = vec![0xC8]; // INY
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.y = 0x41;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.y & 0xFF, 0x42);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_dex() {
        let code = vec![0xCA]; // DEX
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.x = 0x42;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.x & 0xFF, 0x41);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_dey() {
        let code = vec![0x88]; // DEY
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.y = 0x42;
        cpu.p.x = true;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.y & 0xFF, 0x41);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_adc_16bit() {
        let code = vec![0x69, 0x34, 0x12]; // ADC #$1234
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x1000;
        cpu.p.m = false; // 16-bit mode
        cpu.p.c = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a, 0x2234);
        assert!(!cpu.p.c);
        assert!(!cpu.p.v);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
    
    #[test]
    fn test_and_16bit() {
        let code = vec![0x29, 0xFF, 0x00]; // AND #$00FF
        let (mut cpu, mut memory) = create_test_system_with_code(&code);
        
        cpu.a = 0x1234;
        cpu.p.m = false;
        cpu.pc = 0x8000;
        
        cpu.step(&mut memory);
        
        assert_eq!(cpu.a, 0x0034);
        assert!(!cpu.p.z);
        assert!(!cpu.p.n);
    }
}
