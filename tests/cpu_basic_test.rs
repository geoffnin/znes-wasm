/// CPU Basic Tests with Embedded ROM Builder
/// 
/// This module provides a complete SNES LoROM builder for testing CPU functionality
/// with hand-assembled machine code and proper SNES ROM headers.

use znes_wasm::cpu::Cpu65816;
use znes_wasm::memory::Memory;
use znes_wasm::cartridge::Cartridge;

// ============================================================================
// OPCODE CONSTANTS
// ============================================================================

// Load/Store Operations
const LDA_IMM: u8 = 0xA9;      // LDA #immediate
const LDA_ABS: u8 = 0xAD;      // LDA absolute
const LDX_IMM: u8 = 0xA2;      // LDX #immediate
const LDY_IMM: u8 = 0xA0;      // LDY #immediate
const STA_ABS: u8 = 0x8D;      // STA absolute
const STX_ABS: u8 = 0x8E;      // STX absolute
const STY_ABS: u8 = 0x8C;      // STY absolute

// Transfer Operations
const TAX: u8 = 0xAA;          // Transfer A to X
const TAY: u8 = 0xA8;          // Transfer A to Y
const TXA: u8 = 0x8A;          // Transfer X to A
const TYA: u8 = 0x98;          // Transfer Y to A

// Stack Operations
const PHA: u8 = 0x48;          // Push A
const PLA: u8 = 0x68;          // Pull A
const PHP: u8 = 0x08;          // Push Processor Status
const PLP: u8 = 0x28;          // Pull Processor Status

// Arithmetic Operations
const ADC_IMM: u8 = 0x69;      // ADC #immediate
const SBC_IMM: u8 = 0xE9;      // SBC #immediate
const INC_A: u8 = 0x1A;        // INC A
const DEC_A: u8 = 0x3A;        // DEC A
const INX: u8 = 0xE8;          // INX
const INY: u8 = 0xC8;          // INY
const DEX: u8 = 0xCA;          // DEX
const DEY: u8 = 0x88;          // DEY

// Logical Operations
const AND_IMM: u8 = 0x29;      // AND #immediate
const ORA_IMM: u8 = 0x09;      // ORA #immediate
const EOR_IMM: u8 = 0x49;      // EOR #immediate

// Shift/Rotate Operations
const ASL_A: u8 = 0x0A;        // ASL A
const LSR_A: u8 = 0x4A;        // LSR A
const ROL_A: u8 = 0x2A;        // ROL A
const ROR_A: u8 = 0x6A;        // ROR A

// Branch Operations
const BCC: u8 = 0x90;          // Branch if Carry Clear
const BCS: u8 = 0xB0;          // Branch if Carry Set
const BEQ: u8 = 0xF0;          // Branch if Equal (Z=1)
const BNE: u8 = 0xD0;          // Branch if Not Equal (Z=0)
const BMI: u8 = 0x30;          // Branch if Minus (N=1)
const BPL: u8 = 0x10;          // Branch if Plus (N=0)
const BVC: u8 = 0x50;          // Branch if Overflow Clear
const BVS: u8 = 0x70;          // Branch if Overflow Set

// Flag Operations
const CLC: u8 = 0x18;          // Clear Carry
const SEC: u8 = 0x38;          // Set Carry
const CLI: u8 = 0x58;          // Clear Interrupt Disable
const SEI: u8 = 0x78;          // Set Interrupt Disable
const CLV: u8 = 0xB8;          // Clear Overflow
const CLD: u8 = 0xD8;          // Clear Decimal
const SED: u8 = 0xF8;          // Set Decimal

// Comparison Operations
const CMP_IMM: u8 = 0xC9;      // CMP #immediate
const CPX_IMM: u8 = 0xE0;      // CPX #immediate
const CPY_IMM: u8 = 0xC0;      // CPY #immediate

// System Operations
const NOP: u8 = 0xEA;          // No Operation
const BRK: u8 = 0x00;          // Break
const STP: u8 = 0xDB;          // Stop Processor
const WAI: u8 = 0xCB;          // Wait for Interrupt

// 65816 Specific
const REP: u8 = 0xC2;          // Reset Status Bits
const SEP: u8 = 0xE2;          // Set Status Bits
const XCE: u8 = 0xFB;          // Exchange Carry and Emulation bits

// ============================================================================
// TEST METADATA
// ============================================================================

/// Test information structure
#[derive(Debug, Clone)]
pub struct TestInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub expected_cycles: u64,
}

/// Array of test metadata
pub const TEST_METADATA: &[TestInfo] = &[
    TestInfo {
        name: "load_immediate",
        description: "Tests LDA/LDX/LDY with immediate addressing",
        expected_cycles: 6,
    },
    TestInfo {
        name: "transfer_registers",
        description: "Tests register transfer operations (TAX, TAY, TXA, TYA)",
        expected_cycles: 8,
    },
    TestInfo {
        name: "arithmetic_basic",
        description: "Tests ADC and SBC with immediate values",
        expected_cycles: 8,
    },
    TestInfo {
        name: "increment_decrement",
        description: "Tests INC/DEC operations on registers",
        expected_cycles: 12,
    },
    TestInfo {
        name: "logical_operations",
        description: "Tests AND, ORA, EOR with immediate values",
        expected_cycles: 9,
    },
    TestInfo {
        name: "flag_operations",
        description: "Tests flag manipulation (CLC, SEC, etc.)",
        expected_cycles: 12,
    },
    TestInfo {
        name: "comparison",
        description: "Tests CMP, CPX, CPY operations",
        expected_cycles: 9,
    },
    TestInfo {
        name: "stack_operations",
        description: "Tests PHA/PLA and PHP/PLP",
        expected_cycles: 16,
    },
];

// ============================================================================
// ROM BUILDER
// ============================================================================

/// ROM Builder for SNES LoROM format
pub struct RomBuilder {
    rom: Vec<u8>,
}

impl RomBuilder {
    /// Create a new ROM builder with specified size
    pub fn new(size: usize) -> Self {
        Self {
            rom: vec![0xFF; size],
        }
    }
    
    /// Write a byte at the specified address
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < self.rom.len() {
            self.rom[addr] = value;
        }
    }
    
    /// Write a 16-bit value (little-endian)
    pub fn write_word(&mut self, addr: usize, value: u16) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr + 1, (value >> 8) as u8);
    }
    
    /// Write a sequence of bytes
    pub fn write_bytes(&mut self, addr: usize, bytes: &[u8]) {
        for (i, &byte) in bytes.iter().enumerate() {
            self.write_byte(addr + i, byte);
        }
    }
    
    /// Write a null-terminated string
    pub fn write_string(&mut self, addr: usize, s: &str, max_len: usize) {
        let bytes = s.as_bytes();
        let len = bytes.len().min(max_len);
        for i in 0..len {
            self.write_byte(addr + i, bytes[i]);
        }
        // Pad with spaces
        for i in len..max_len {
            self.write_byte(addr + i, 0x20);
        }
    }
    
    /// Build and return the ROM
    pub fn build(self) -> Vec<u8> {
        self.rom
    }
}

/// Write LoROM header at $7FC0
fn write_lorom_header(builder: &mut RomBuilder, title: &str) {
    // Header starts at $7FC0 in LoROM
    let header_base = 0x7FC0;
    
    // Write game title (21 bytes, padded with spaces)
    builder.write_string(header_base, title, 21);
    
    // ROM makeup byte ($7FD5)
    // Bit 0: ROM speed (0=slow, 1=fast)
    // Bits 4-7: Map mode (0=LoROM, 1=HiROM, etc.)
    builder.write_byte(header_base + 0x15, 0x20); // LoROM, slow
    
    // Chipset type ($7FD6) - 0 = ROM only
    builder.write_byte(header_base + 0x16, 0x00);
    
    // ROM size ($7FD7) - $09 = 512KB (2^9 * 1024)
    builder.write_byte(header_base + 0x17, 0x09);
    
    // SRAM size ($7FD8) - 0 = no SRAM
    builder.write_byte(header_base + 0x18, 0x00);
    
    // Country code ($7FD9) - 1 = USA
    builder.write_byte(header_base + 0x19, 0x01);
    
    // Developer ID ($7FDA)
    builder.write_byte(header_base + 0x1A, 0x00);
    
    // Version ($7FDB)
    builder.write_byte(header_base + 0x1B, 0x00);
    
    // Checksum complement will be calculated later ($7FDC-$7FDD)
    // Checksum will be calculated later ($7FDE-$7FDF)
    
    // Interrupt vectors (in LoROM, bank $00 at $FFE4-$FFFF maps to ROM $7FE4-$7FFF)
    // Native mode vectors
    builder.write_word(0x7FE4, 0x0000); // COP
    builder.write_word(0x7FE6, 0x0000); // BRK
    builder.write_word(0x7FE8, 0x0000); // ABORT
    builder.write_word(0x7FEA, 0x0000); // NMI
    builder.write_word(0x7FEC, 0x0000); // Unused
    builder.write_word(0x7FEE, 0x0000); // IRQ
    
    // Emulation mode vectors
    builder.write_word(0x7FF4, 0x0000); // COP
    builder.write_word(0x7FF6, 0x0000); // Unused
    builder.write_word(0x7FF8, 0x0000); // ABORT
    builder.write_word(0x7FFA, 0x0000); // NMI
    builder.write_word(0x7FFC, 0x8000); // RESET - points to $8000
    builder.write_word(0x7FFE, 0x0000); // IRQ/BRK
}

/// Calculate and write checksum
fn calculate_checksum(builder: &mut RomBuilder) {
    let mut sum: u32 = 0;
    
    // Sum all bytes in the ROM
    for &byte in builder.rom.iter() {
        sum = sum.wrapping_add(byte as u32);
    }
    
    // The checksum is the lower 16 bits
    let checksum = (sum & 0xFFFF) as u16;
    let checksum_complement = !checksum;
    
    // Write checksum complement at $7FDC
    builder.write_word(0x7FDC, checksum_complement);
    
    // Write checksum at $7FDE
    builder.write_word(0x7FDE, checksum);
}

/// Assemble a test sequence at the specified address
fn assemble_test_sequence(builder: &mut RomBuilder, start_addr: usize, sequence: &[u8]) {
    builder.write_bytes(start_addr, sequence);
}

/// Build a complete test ROM with the given test code
pub fn build_test_rom(test_code: &[u8]) -> Vec<u8> {
    // Create a 512KB ROM (standard LoROM size)
    let mut builder = RomBuilder::new(512 * 1024);
    
    // Write the LoROM header
    write_lorom_header(&mut builder, "CPU BASIC TEST");
    
    // Write test code at ROM offset $0000
    // In LoROM, bank $00 at $8000 maps to ROM offset $0000
    assemble_test_sequence(&mut builder, 0x0000, test_code);
    
    // Calculate and write checksum
    calculate_checksum(&mut builder);
    
    builder.build()
}

/// Build a simple test ROM with basic instructions
pub fn build_simple_test_rom() -> Vec<u8> {
    let test_code = vec![
        // Test at $8000
        LDA_IMM, 0x42,     // LDA #$42
        LDX_IMM, 0x10,     // LDX #$10
        LDY_IMM, 0x20,     // LDY #$20
        NOP,               // NOP
        STP,               // Stop processor
    ];
    
    build_test_rom(&test_code)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Helper function to create a CPU with test ROM
    fn setup_cpu_with_rom(rom_data: Vec<u8>) -> (Cpu65816, Memory) {
        let cartridge = Cartridge::from_rom(rom_data).expect("Failed to load ROM");
        let mut memory = Memory::new(&cartridge);
        let mut cpu = Cpu65816::new();
        cpu.reset(&mut memory);
        (cpu, memory)
    }
    
    #[test]
    fn test_rom_builder_creates_valid_header() {
        let rom = build_simple_test_rom();
        
        // Check ROM size
        assert_eq!(rom.len(), 512 * 1024);
        
        // Check title
        let title_start = 0x7FC0;
        let title_bytes = &rom[title_start..title_start + 21];
        let title = std::str::from_utf8(title_bytes)
            .unwrap_or("")
            .trim();
        assert!(title.starts_with("CPU BASIC TEST"));
        
        // Check ROM makeup byte
        assert_eq!(rom[0x7FD5], 0x20); // LoROM, slow
        
        // Check ROM size byte
        assert_eq!(rom[0x7FD7], 0x09); // 512KB
        
        // Check reset vector points to $8000 (at ROM offset $7FFC for LoROM)
        let reset_vector = (rom[0x7FFC] as u16) | ((rom[0x7FFD] as u16) << 8);
        assert_eq!(reset_vector, 0x8000);
    }
    
    #[test]
    fn test_rom_builder_checksum() {
        let rom = build_simple_test_rom();
        
        // Read stored checksum
        let stored_checksum = (rom[0x7FDE] as u16) | ((rom[0x7FDF] as u16) << 8);
        let stored_complement = (rom[0x7FDC] as u16) | ((rom[0x7FDD] as u16) << 8);
        
        // Verify checksum and complement are inverses
        assert_eq!(stored_checksum, !stored_complement);
    }
    
    #[test]
    fn test_load_immediate_operations() {
        let test_code = vec![
            LDA_IMM, 0x42,     // LDA #$42
            LDX_IMM, 0x55,     // LDX #$55
            LDY_IMM, 0xAA,     // LDY #$AA
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        // Execute LDA
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x42);
        
        // Execute LDX
        cpu.step(&mut memory);
        assert_eq!(cpu.x & 0xFF, 0x55);
        
        // Execute LDY
        cpu.step(&mut memory);
        assert_eq!(cpu.y & 0xFF, 0xAA);
    }
    
    #[test]
    fn test_transfer_operations() {
        let test_code = vec![
            LDA_IMM, 0x42,     // LDA #$42
            TAX,               // TAX (A -> X)
            TXA,               // TXA (X -> A)
            TAY,               // TAY (A -> Y)
            TYA,               // TYA (Y -> A)
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        // Execute LDA #$42
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x42);
        
        // Execute TAX
        cpu.step(&mut memory);
        assert_eq!(cpu.x & 0xFF, 0x42);
        
        // Execute TXA
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x42);
        
        // Execute TAY
        cpu.step(&mut memory);
        assert_eq!(cpu.y & 0xFF, 0x42);
        
        // Execute TYA
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x42);
    }
    
    #[test]
    fn test_arithmetic_operations() {
        let test_code = vec![
            CLC,               // Clear carry
            LDA_IMM, 0x10,     // LDA #$10
            ADC_IMM, 0x20,     // ADC #$20 -> A = $30
            SBC_IMM, 0x05,     // SBC #$05 -> A = $2B (with carry behavior)
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        // Execute CLC
        cpu.step(&mut memory);
        assert!(!cpu.p.c);
        
        // Execute LDA
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x10);
        
        // Execute ADC
        cpu.step(&mut memory);
        assert_eq!(cpu.a & 0xFF, 0x30);
    }
    
    #[test]
    fn test_increment_decrement() {
        let test_code = vec![
            LDA_IMM, 0x10,     // LDA #$10
            INC_A,             // INC A -> $11
            INC_A,             // INC A -> $12
            DEC_A,             // DEC A -> $11
            LDX_IMM, 0x05,     // LDX #$05
            INX,               // INX -> $06
            DEX,               // DEX -> $05
            DEX,               // DEX -> $04
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        cpu.step(&mut memory); // LDA
        assert_eq!(cpu.a & 0xFF, 0x10);
        
        cpu.step(&mut memory); // INC A
        assert_eq!(cpu.a & 0xFF, 0x11);
        
        cpu.step(&mut memory); // INC A
        assert_eq!(cpu.a & 0xFF, 0x12);
        
        cpu.step(&mut memory); // DEC A
        assert_eq!(cpu.a & 0xFF, 0x11);
        
        cpu.step(&mut memory); // LDX
        assert_eq!(cpu.x & 0xFF, 0x05);
        
        cpu.step(&mut memory); // INX
        assert_eq!(cpu.x & 0xFF, 0x06);
        
        cpu.step(&mut memory); // DEX
        assert_eq!(cpu.x & 0xFF, 0x05);
        
        cpu.step(&mut memory); // DEX
        assert_eq!(cpu.x & 0xFF, 0x04);
    }
    
    #[test]
    fn test_logical_operations() {
        let test_code = vec![
            LDA_IMM, 0xFF,     // LDA #$FF
            AND_IMM, 0x0F,     // AND #$0F -> $0F
            ORA_IMM, 0xF0,     // ORA #$F0 -> $FF
            EOR_IMM, 0xAA,     // EOR #$AA -> $55
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        cpu.step(&mut memory); // LDA
        assert_eq!(cpu.a & 0xFF, 0xFF);
        
        cpu.step(&mut memory); // AND
        assert_eq!(cpu.a & 0xFF, 0x0F);
        
        cpu.step(&mut memory); // ORA
        assert_eq!(cpu.a & 0xFF, 0xFF);
        
        cpu.step(&mut memory); // EOR
        assert_eq!(cpu.a & 0xFF, 0x55);
    }
    
    #[test]
    fn test_flag_operations() {
        let test_code = vec![
            CLC,               // Clear carry
            SEC,               // Set carry
            CLC,               // Clear carry
            CLV,               // Clear overflow
            CLD,               // Clear decimal
            SEI,               // Set interrupt disable
            CLI,               // Clear interrupt disable
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        cpu.step(&mut memory); // CLC
        assert!(!cpu.p.c);
        
        cpu.step(&mut memory); // SEC
        assert!(cpu.p.c);
        
        cpu.step(&mut memory); // CLC
        assert!(!cpu.p.c);
        
        cpu.step(&mut memory); // CLV
        assert!(!cpu.p.v);
        
        cpu.step(&mut memory); // CLD
        assert!(!cpu.p.d);
    }
    
    #[test]
    fn test_comparison_operations() {
        let test_code = vec![
            LDA_IMM, 0x42,     // LDA #$42
            CMP_IMM, 0x42,     // CMP #$42 (equal, Z=1, C=1)
            CMP_IMM, 0x40,     // CMP #$40 (A > M, C=1, Z=0)
            LDX_IMM, 0x10,     // LDX #$10
            CPX_IMM, 0x20,     // CPX #$20 (X < M, C=0)
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        cpu.step(&mut memory); // LDA
        assert_eq!(cpu.a & 0xFF, 0x42);
        
        cpu.step(&mut memory); // CMP #$42
        assert!(cpu.p.z);      // Equal
        assert!(cpu.p.c);      // A >= M
        
        cpu.step(&mut memory); // CMP #$40
        assert!(!cpu.p.z);     // Not equal
        assert!(cpu.p.c);      // A >= M
    }
    
    #[test]
    fn test_metadata_array() {
        // Verify test metadata is accessible
        assert!(TEST_METADATA.len() > 0);
        assert_eq!(TEST_METADATA[0].name, "load_immediate");
        assert!(TEST_METADATA[0].description.len() > 0);
    }
}
