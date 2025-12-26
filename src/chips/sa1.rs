/// SA-1 Coprocessor Implementation
///
/// The SA-1 (Super Accelerator 1) is essentially a second 65816 CPU running at 10.74 MHz
/// with additional features including:
/// - 2KB internal RAM (I-RAM)
/// - Bitmap conversion hardware
/// - Variable-length bit processing
/// - Memory mapping override
/// - DMA capabilities
///
/// Used in games like Super Mario RPG, Kirby Super Star, and Kirby's Dream Land 3.
///
/// Memory Map:
/// - 0x2200-0x23FF: SA-1 CPU Registers
/// - 0x3000-0x37FF: I-RAM (2KB internal RAM)
/// - Bank remapping for ROM/RAM access

use super::CoProcessor;

/// SA-1 Communication Registers
#[derive(Debug, Default)]
struct Sa1Registers {
    // Control registers
    ccnt: u8,           // 0x2200: SA-1 Control
    sie: u8,            // 0x2201: SNES Interrupt Enable
    sic: u8,            // 0x2202: SNES Interrupt Clear
    
    // CPU communication
    crv: u16,           // 0x2203-0x2204: SA-1 Reset Vector
    cnv: u16,           // 0x2205-0x2206: SA-1 NMI Vector
    civ: u16,           // 0x2207-0x2208: SA-1 IRQ Vector
    
    // Status
    scnt: u8,           // 0x2209: SNES Control
    cie: u8,            // 0x220A: SA-1 Interrupt Enable
    cic: u8,            // 0x220B: SA-1 Interrupt Clear
    
    // SNES CPU communication
    snv: u16,           // 0x220C-0x220D: SNES NMI Vector
    siv: u16,           // 0x220E-0x220F: SNES IRQ Vector
    
    // Message passing (4 bytes each direction)
    cfr: u8,            // 0x2210: SA-1 Flag Read
    hcr: u8,            // 0x2211: H-Count Read (H-IRQ position)
    vcr: u8,            // 0x2212: V-Count Read (V-IRQ position)
    
    // Memory mapping
    cxb: u8,            // 0x2220: Character Conversion DMA Parameters
    dxb: u8,            // 0x2221: DMA Parameters
    exb: u8,            // 0x2222: DMA Parameters
    fxb: u8,            // 0x2223: DMA Parameters
    
    bmaps: u8,          // 0x2224: Bitmap register
    bmap: u8,           // 0x2225: Bitmap register (file)
    sbwe: u8,           // 0x2226: S-CPU BW-RAM Write Enable
    cbwe: u8,           // 0x2227: SA-1 BW-RAM Write Enable
    bwpa: u8,           // 0x2228: BW-RAM Write-Protected Area
    siwp: u8,           // 0x2229: S-CPU I-RAM Write Protection
    ciwp: u8,           // 0x222A: SA-1 I-RAM Write Protection
    
    // DMA control
    dcnt: u8,           // 0x2230: DMA Control
    cdma: u8,           // 0x2231: Character Conversion DMA Control
    
    // Source/Destination addresses
    sda: u32,           // 0x2232-0x2234: DMA Source Address
    dda: u16,           // 0x2235-0x2236: DMA Destination Address (internal I-RAM)
    dtc: u16,           // 0x2238-0x2239: DMA Terminal Counter
    
    // Bitmap conversion
    brf: u8,            // 0x223F: Bitmap Register File
    
    // Math registers
    math_a: u16,        // 0x2250-0x2251: Multiplicand/Dividend
    math_b: u16,        // 0x2252-0x2253: Multiplier/Divisor
    
    // Variable-length bit processing
    vbd: u8,            // 0x2258: Variable-Length Bit Processing
    vda: u32,           // 0x2259-0x225B: Variable-Length Bit Processing Address
    
    // Message passing buffers
    snes_message: [u8; 4],  // 0x2300-0x2303: SNES -> SA-1
    sa1_message: [u8; 4],   // 0x2304-0x2307: SA-1 -> SNES
}

/// SA-1 Coprocessor State
pub struct Sa1 {
    /// SA-1 CPU registers (simplified - full 65816 implementation would go here)
    /// For now, we'll use a cycle counter and stub execution
    sa1_cycles: u64,
    sa1_running: bool,
    sa1_pc: u16,
    sa1_a: u16,
    sa1_x: u16,
    sa1_y: u16,
    
    /// 2KB Internal RAM
    iram: Box<[u8; 0x800]>,
    
    /// Communication registers
    registers: Sa1Registers,
    
    /// IRQ/NMI pending flags
    sa1_irq_pending: bool,
    sa1_nmi_pending: bool,
    snes_irq_pending: bool,
    snes_nmi_pending: bool,
    
    /// Math operation results
    math_result: u64,
    
    /// Variable-length bit processing state
    vbit_buffer: u8,
    vbit_count: u8,
}

impl Sa1 {
    pub fn new() -> Self {
        Self {
            sa1_cycles: 0,
            sa1_running: false,
            sa1_pc: 0,
            sa1_a: 0,
            sa1_x: 0,
            sa1_y: 0,
            iram: Box::new([0; 0x800]),
            registers: Sa1Registers::default(),
            sa1_irq_pending: false,
            sa1_nmi_pending: false,
            snes_irq_pending: false,
            snes_nmi_pending: false,
            math_result: 0,
            vbit_buffer: 0,
            vbit_count: 0,
        }
    }

    /// Read from SA-1 register space
    fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            // Status flags
            0x2300 => {
                // SA-1 Status
                let mut status = 0u8;
                if self.sa1_running { status |= 0x80; }
                if self.sa1_irq_pending { status |= 0x40; }
                if self.sa1_nmi_pending { status |= 0x20; }
                status
            }
            
            // Message passing: SA-1 -> SNES
            0x2301..=0x2304 => {
                let idx = (addr - 0x2301) as usize;
                self.registers.sa1_message[idx]
            }
            
            // Math multiply result (48-bit) - overlaps with above, handle separately
            0x2306 => (self.math_result & 0xFF) as u8,
            0x2307 => ((self.math_result >> 8) & 0xFF) as u8,
            0x2308 => ((self.math_result >> 16) & 0xFF) as u8,
            0x2309 => ((self.math_result >> 24) & 0xFF) as u8,
            0x230A => ((self.math_result >> 32) & 0xFF) as u8,
            0x230B => ((self.math_result >> 40) & 0xFF) as u8,
            
            // Math divide result
            0x230C => (self.math_result & 0xFF) as u8,
            0x230D => ((self.math_result >> 8) & 0xFF) as u8,
            
            // Math remainder
            0x230E => ((self.math_result >> 16) & 0xFF) as u8,
            0x230F => ((self.math_result >> 24) & 0xFF) as u8,
            
            // Variable-length bit read
            0x2231 => {
                let bit = if self.vbit_count > 0 {
                    let result = (self.vbit_buffer >> 7) & 1;
                    self.vbit_buffer <<= 1;
                    self.vbit_count -= 1;
                    result
                } else {
                    0
                };
                bit
            }
            
            _ => 0,
        }
    }

    /// Write to SA-1 register space
    fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            // SA-1 Control
            0x2200 => {
                self.registers.ccnt = val;
                // Bit 7: SA-1 Reset
                // Bit 5: SA-1 IRQ Enable
                // Bit 4: SA-1 Wait
                if val & 0x80 != 0 {
                    self.sa1_running = true;
                    self.sa1_pc = self.registers.crv;
                }
                if val & 0x20 != 0 {
                    self.sa1_irq_pending = true;
                }
            }
            
            // SNES Interrupt Enable
            0x2201 => {
                self.registers.sie = val;
            }
            
            // SNES Interrupt Clear
            0x2202 => {
                if val & 0x80 != 0 { self.snes_nmi_pending = false; }
                if val & 0x40 != 0 { self.sa1_nmi_pending = false; }
            }
            
            // SA-1 Reset Vector
            0x2203 => self.registers.crv = (self.registers.crv & 0xFF00) | val as u16,
            0x2204 => self.registers.crv = (self.registers.crv & 0x00FF) | ((val as u16) << 8),
            
            // SA-1 NMI Vector
            0x2205 => self.registers.cnv = (self.registers.cnv & 0xFF00) | val as u16,
            0x2206 => self.registers.cnv = (self.registers.cnv & 0x00FF) | ((val as u16) << 8),
            
            // SA-1 IRQ Vector
            0x2207 => self.registers.civ = (self.registers.civ & 0xFF00) | val as u16,
            0x2208 => self.registers.civ = (self.registers.civ & 0x00FF) | ((val as u16) << 8),
            
            // SNES Control
            0x2209 => {
                self.registers.scnt = val;
                if val & 0x80 != 0 {
                    self.snes_irq_pending = true;
                }
            }
            
            // SA-1 Interrupt Enable
            0x220A => {
                self.registers.cie = val;
            }
            
            // SA-1 Interrupt Clear
            0x220B => {
                if val & 0x80 != 0 { self.sa1_nmi_pending = false; }
                if val & 0x40 != 0 { self.sa1_irq_pending = false; }
            }
            
            // SNES NMI Vector
            0x220C => self.registers.snv = (self.registers.snv & 0xFF00) | val as u16,
            0x220D => self.registers.snv = (self.registers.snv & 0x00FF) | ((val as u16) << 8),
            
            // SNES IRQ Vector
            0x220E => self.registers.siv = (self.registers.siv & 0xFF00) | val as u16,
            0x220F => self.registers.siv = (self.registers.siv & 0x00FF) | ((val as u16) << 8),
            
            // Message passing: SNES -> SA-1
            0x2210..=0x2213 => {
                let idx = (addr - 0x2210) as usize;
                self.registers.snes_message[idx] = val;
            }
            
            // Message passing: SA-1 -> SNES
            0x2214..=0x2217 => {
                let idx = (addr - 0x2214) as usize;
                self.registers.sa1_message[idx] = val;
            }
            
            // Memory mapping registers
            0x2220..=0x222A => {
                match addr {
                    0x2220 => self.registers.cxb = val,
                    0x2221 => self.registers.dxb = val,
                    0x2222 => self.registers.exb = val,
                    0x2223 => self.registers.fxb = val,
                    0x2224 => self.registers.bmaps = val,
                    0x2225 => self.registers.bmap = val,
                    0x2226 => self.registers.sbwe = val,
                    0x2227 => self.registers.cbwe = val,
                    0x2228 => self.registers.bwpa = val,
                    0x2229 => self.registers.siwp = val,
                    0x222A => self.registers.ciwp = val,
                    _ => {}
                }
            }
            
            // DMA Control
            0x2230 => {
                self.registers.dcnt = val;
                // Bit 7: DMA Enable
                // Bit 2-0: DMA Type
                if val & 0x80 != 0 {
                    self.execute_dma();
                }
            }
            
            // Character Conversion DMA
            0x2231 => {
                self.registers.cdma = val;
                if val & 0x80 != 0 {
                    self.execute_character_conversion();
                }
            }
            
            // DMA Source Address
            0x2232 => self.registers.sda = (self.registers.sda & 0xFFFF00) | val as u32,
            0x2233 => self.registers.sda = (self.registers.sda & 0xFF00FF) | ((val as u32) << 8),
            0x2234 => self.registers.sda = (self.registers.sda & 0x00FFFF) | ((val as u32) << 16),
            
            // DMA Destination Address
            0x2235 => self.registers.dda = (self.registers.dda & 0xFF00) | val as u16,
            0x2236 => self.registers.dda = (self.registers.dda & 0x00FF) | ((val as u16) << 8),
            
            // DMA Terminal Counter (length)
            0x2238 => self.registers.dtc = (self.registers.dtc & 0xFF00) | val as u16,
            0x2239 => self.registers.dtc = (self.registers.dtc & 0x00FF) | ((val as u16) << 8),
            
            // Bitmap Register File
            0x223F => self.registers.brf = val,
            
            // Math: Multiplicand/Dividend
            0x2250 => {
                self.registers.math_a = (self.registers.math_a & 0xFF00) | val as u16;
            }
            0x2251 => {
                self.registers.math_a = (self.registers.math_a & 0x00FF) | ((val as u16) << 8);
            }
            
            // Math: Multiplier/Divisor (triggers operation)
            0x2252 => {
                self.registers.math_b = (self.registers.math_b & 0xFF00) | val as u16;
            }
            0x2253 => {
                self.registers.math_b = (self.registers.math_b & 0x00FF) | ((val as u16) << 8);
                // Trigger math operation based on control register
                self.execute_math_operation();
            }
            
            // Variable-length bit processing
            0x2258 => {
                self.registers.vbd = val;
                // Load bits into buffer
                self.vbit_buffer = val;
                self.vbit_count = 8;
            }
            
            0x2259 => self.registers.vda = (self.registers.vda & 0xFFFF00) | val as u32,
            0x225A => self.registers.vda = (self.registers.vda & 0xFF00FF) | ((val as u32) << 8),
            0x225B => self.registers.vda = (self.registers.vda & 0x00FFFF) | ((val as u32) << 16),
            
            _ => {}
        }
    }

    /// Execute DMA transfer
    fn execute_dma(&mut self) {
        // Simplified DMA: transfer from ROM/RAM to I-RAM
        let _src = self.registers.sda;
        let dst = self.registers.dda & 0x7FF; // I-RAM is only 2KB
        let len = self.registers.dtc.min(0x800 - dst);
        
        // In a real implementation, this would read from the ROM/RAM
        // For now, just fill with zeros
        for i in 0..len {
            let dst_addr = (dst + i) as usize;
            if dst_addr < 0x800 {
                self.iram[dst_addr] = 0; // Placeholder
            }
        }
    }

    /// Execute character conversion (bitmap conversion)
    fn execute_character_conversion(&mut self) {
        // Character conversion converts linear bitmap data to SNES tile format
        // This is a simplified implementation
        let _bpp = (self.registers.cdma & 0x03) + 1; // Bits per pixel (1-4)
        
        // In a real implementation, this would:
        // 1. Read linear bitmap data
        // 2. Convert to SNES planar format
        // 3. Write to destination
        
        // Placeholder for now
    }

    /// Execute math operation (multiply or divide)
    fn execute_math_operation(&mut self) {
        // Determine operation type from control register
        let a = self.registers.math_a as i16;
        let b = self.registers.math_b as i16;
        
        if self.registers.ccnt & 0x01 != 0 {
            // Signed division
            if b != 0 {
                let quotient = a / b;
                let remainder = a % b;
                self.math_result = ((remainder as u16 as u64) << 16) | (quotient as u16 as u64);
            } else {
                self.math_result = 0;
            }
        } else {
            // Signed multiplication
            let result = (a as i32) * (b as i32);
            self.math_result = result as u64;
        }
    }

    /// Execute SA-1 CPU cycles (simplified)
    fn execute_sa1(&mut self, cycles: u32) {
        if !self.sa1_running {
            return;
        }
        
        // In a full implementation, this would execute 65816 instructions
        // For now, we just increment the cycle counter
        self.sa1_cycles += cycles as u64;
        
        // Simplified execution: advance PC
        self.sa1_pc = self.sa1_pc.wrapping_add(1);
    }
}

impl CoProcessor for Sa1 {
    fn reset(&mut self) {
        self.sa1_cycles = 0;
        self.sa1_running = false;
        self.sa1_pc = 0;
        self.sa1_a = 0;
        self.sa1_x = 0;
        self.sa1_y = 0;
        self.iram.fill(0);
        self.registers = Sa1Registers::default();
        self.sa1_irq_pending = false;
        self.sa1_nmi_pending = false;
        self.snes_irq_pending = false;
        self.snes_nmi_pending = false;
        self.math_result = 0;
        self.vbit_buffer = 0;
        self.vbit_count = 0;
    }

    fn read(&mut self, addr: u32) -> u8 {
        let addr = addr & 0xFFFFFF;
        
        match addr {
            // I-RAM: 0x3000-0x37FF
            0x003000..=0x0037FF => {
                let offset = (addr - 0x3000) as usize;
                self.iram[offset]
            }
            
            // SA-1 Registers: 0x2200-0x23FF
            0x002200..=0x0023FF => {
                self.read_register((addr & 0xFFFF) as u16)
            }
            
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, val: u8) {
        let addr = addr & 0xFFFFFF;
        
        match addr {
            // I-RAM: 0x3000-0x37FF
            0x003000..=0x0037FF => {
                let offset = (addr - 0x3000) as usize;
                // Check write protection
                if self.registers.ciwp & 0x80 == 0 || offset >= 0x100 {
                    self.iram[offset] = val;
                }
            }
            
            // SA-1 Registers: 0x2200-0x23FF
            0x002200..=0x0023FF => {
                self.write_register((addr & 0xFFFF) as u16, val);
            }
            
            _ => {}
        }
    }

    fn step(&mut self, cycles: u32) -> u32 {
        // SA-1 runs at 10.74 MHz (same as main CPU)
        // Execute SA-1 CPU if running
        self.execute_sa1(cycles);
        cycles
    }

    fn handles_address(&self, addr: u32) -> bool {
        let addr = addr & 0xFFFFFF;
        matches!(addr, 0x002200..=0x0023FF | 0x003000..=0x0037FF)
    }
}

impl Default for Sa1 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iram_access() {
        let mut sa1 = Sa1::new();
        
        // Write to I-RAM
        sa1.write(0x3000, 0x42);
        sa1.write(0x37FF, 0xFF);
        
        // Read back
        assert_eq!(sa1.read(0x3000), 0x42);
        assert_eq!(sa1.read(0x37FF), 0xFF);
    }

    #[test]
    fn test_message_passing() {
        let mut sa1 = Sa1::new();
        
        // SNES writes to SA-1
        sa1.write(0x2210, 0x12);
        sa1.write(0x2211, 0x34);
        
        // SA-1 reads from SNES
        assert_eq!(sa1.registers.snes_message[0], 0x12);
        assert_eq!(sa1.registers.snes_message[1], 0x34);
        
        // SA-1 writes back to SNES
        sa1.write(0x2214, 0xAB);
        sa1.write(0x2215, 0xCD);
        
        assert_eq!(sa1.read(0x2301), 0xAB);
        assert_eq!(sa1.read(0x2302), 0xCD);
    }

    #[test]
    fn test_math_multiply() {
        let mut sa1 = Sa1::new();
        
        // Set up multiplication: 100 * 200
        sa1.write(0x2250, 100);  // Low byte of A
        sa1.write(0x2251, 0);    // High byte of A
        sa1.write(0x2252, 200);  // Low byte of B
        sa1.write(0x2253, 0);    // High byte of B (triggers operation)
        
        // Read result (should be 20000 = 0x4E20)
        let lo = sa1.read(0x2306) as u32;
        let hi = sa1.read(0x2307) as u32;
        let result = (hi << 8) | lo;
        
        assert_eq!(result, 20000);
    }

    #[test]
    fn test_handles_address() {
        let sa1 = Sa1::new();
        
        assert!(sa1.handles_address(0x2200));
        assert!(sa1.handles_address(0x23FF));
        assert!(sa1.handles_address(0x3000));
        assert!(sa1.handles_address(0x37FF));
        assert!(!sa1.handles_address(0x2000));
        assert!(!sa1.handles_address(0x4000));
    }

    #[test]
    fn test_sa1_control() {
        let mut sa1 = Sa1::new();
        
        // Set reset vector
        sa1.write(0x2203, 0x00);
        sa1.write(0x2204, 0x80);
        assert_eq!(sa1.registers.crv, 0x8000);
        
        // Start SA-1
        sa1.write(0x2200, 0x80);
        assert!(sa1.sa1_running);
        assert_eq!(sa1.sa1_pc, 0x8000);
    }
}
