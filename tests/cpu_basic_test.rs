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

/// Build comprehensive test ROM with ~50 independent test sequences
/// Each test writes results to direct page ($00-$FF which maps to $7E0000-$7E00FF)
pub fn build_comprehensive_test_rom() -> Vec<u8> {
    let mut code = Vec::new();
    
    // Initialization sequence
    code.extend_from_slice(&[
        CLC,               // $18 - Clear carry
        XCE,               // $FB - Exchange carry with emulation (enter native mode)
        SEP, 0x30,         // $E2 $30 - Ensure 8-bit A and X/Y mode for consistency
    ]);
    
    // =================================================================
    // MODE SWITCHING TESTS (Tests 0-2)
    // =================================================================
    
    // Test 0: REP #$20 enables 16-bit accumulator
    code.extend_from_slice(&[
        LDA_IMM, 0x00,     // Reset A
        REP, 0x20,         // Enable 16-bit A
        LDA_IMM, 0x34, 0x12,  // LDA #$1234 (16-bit)
        SEP, 0x20,         // Back to 8-bit A
        0x85, 0x00,        // STA $00 (stores low byte)
    ]);
    
    // Test 1: REP #$10 enables 16-bit X/Y
    code.extend_from_slice(&[
        REP, 0x10,         // Enable 16-bit X/Y
        LDX_IMM, 0x56, 0x78,  // LDX #$7856
        SEP, 0x10,         // Back to 8-bit
        TXA,               // Transfer X to A
        0x85, 0x01,        // STA $01
    ]);
    
    // Test 2: REP #$30 enables both 16-bit A and X/Y
    code.extend_from_slice(&[
        REP, 0x30,         // Enable 16-bit A and X/Y
        LDA_IMM, 0xCD, 0xAB,
        SEP, 0x30,         // Back to 8-bit
        0x85, 0x02,        // STA $02
    ]);
    
    // =================================================================
    // LOAD/STORE IMMEDIATE TESTS (Tests 3-8)
    // =================================================================
    
    // Test 3: LDA immediate 8-bit
    code.extend_from_slice(&[
        SEP, 0x30,         // Ensure 8-bit mode
        LDA_IMM, 0x42,
        0x85, 0x03,        // STA $03
    ]);
    
    // Test 4: LDX immediate 8-bit
    code.extend_from_slice(&[
        LDX_IMM, 0x55,
        TXA,
        0x85, 0x04,        // STA $04
    ]);
    
    // Test 5: LDY immediate 8-bit
    code.extend_from_slice(&[
        LDY_IMM, 0xAA,
        TYA,
        0x85, 0x05,        // STA $05
    ]);
    
    // Test 6: LDA immediate 16-bit
    code.extend_from_slice(&[
        REP, 0x20,         // 16-bit A
        LDA_IMM, 0x34, 0x12,
        SEP, 0x20,
        0x85, 0x06,        // Store low byte
    ]);
    
    // Test 7: LDX immediate 16-bit
    code.extend_from_slice(&[
        REP, 0x10,         // 16-bit X/Y
        LDX_IMM, 0x78, 0x56,
        TXA,
        SEP, 0x10,
        0x85, 0x07,
    ]);
    
    // Test 8: LDY immediate 16-bit
    code.extend_from_slice(&[
        REP, 0x10,
        LDY_IMM, 0xBC, 0x9A,
        TYA,
        SEP, 0x10,
        0x85, 0x08,
    ]);
    
    // =================================================================
    // ARITHMETIC TESTS (Tests 9-18)
    // =================================================================
    
    // Test 9: ADC 8-bit no carry
    code.extend_from_slice(&[
        SEP, 0x30,
        CLC,
        LDA_IMM, 0x10,
        ADC_IMM, 0x20,     // 0x10 + 0x20 = 0x30
        0x85, 0x09,
    ]);
    
    // Test 10: ADC 8-bit with carry
    code.extend_from_slice(&[
        SEC,               // Set carry
        LDA_IMM, 0x10,
        ADC_IMM, 0x20,     // 0x10 + 0x20 + 1 = 0x31
        0x85, 0x0A,
    ]);
    
    // Test 11: SBC 8-bit
    code.extend_from_slice(&[
        SEC,               // Clear borrow
        LDA_IMM, 0x50,
        SBC_IMM, 0x20,     // 0x50 - 0x20 = 0x30
        0x85, 0x0B,
    ]);
    
    // Test 12: ADC 16-bit
    code.extend_from_slice(&[
        CLC,
        REP, 0x20,
        LDA_IMM, 0x00, 0x10,  // $1000
        ADC_IMM, 0x00, 0x20,  // + $2000 = $3000
        SEP, 0x20,
        0x85, 0x0C,        // Store low byte
    ]);
    
    // Test 13: SBC 16-bit
    code.extend_from_slice(&[
        SEC,
        REP, 0x20,
        LDA_IMM, 0x00, 0x50,  // $5000
        SBC_IMM, 0x00, 0x20,  // - $2000 = $3000
        SEP, 0x20,
        0x85, 0x0D,
    ]);
    
    // Test 14: INC A
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x10,
        INC_A,             // 0x10 -> 0x11
        0x85, 0x0E,
    ]);
    
    // Test 15: DEC A
    code.extend_from_slice(&[
        LDA_IMM, 0x20,
        DEC_A,             // 0x20 -> 0x1F
        0x85, 0x0F,
    ]);
    
    // Test 16: INX
    code.extend_from_slice(&[
        LDX_IMM, 0x30,
        INX,               // 0x30 -> 0x31
        TXA,
        0x85, 0x10,
    ]);
    
    // Test 17: DEX
    code.extend_from_slice(&[
        LDX_IMM, 0x40,
        DEX,               // 0x40 -> 0x3F
        TXA,
        0x85, 0x11,
    ]);
    
    // Test 18: INY and DEY
    code.extend_from_slice(&[
        LDY_IMM, 0x50,
        INY,               // 0x50 -> 0x51
        DEY,               // 0x51 -> 0x50
        TYA,
        0x85, 0x12,
    ]);
    
    // =================================================================
    // LOGICAL OPERATIONS (Tests 19-24)
    // =================================================================
    
    // Test 19: AND immediate
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0xFF,
        AND_IMM, 0x0F,     // 0xFF & 0x0F = 0x0F
        0x85, 0x13,
    ]);
    
    // Test 20: ORA immediate
    code.extend_from_slice(&[
        LDA_IMM, 0x0F,
        ORA_IMM, 0xF0,     // 0x0F | 0xF0 = 0xFF
        0x85, 0x14,
    ]);
    
    // Test 21: EOR immediate
    code.extend_from_slice(&[
        LDA_IMM, 0xFF,
        EOR_IMM, 0xAA,     // 0xFF ^ 0xAA = 0x55
        0x85, 0x15,
    ]);
    
    // Test 22: AND 16-bit
    code.extend_from_slice(&[
        REP, 0x20,
        LDA_IMM, 0xFF, 0xFF,
        AND_IMM, 0x0F, 0xF0,
        SEP, 0x20,
        0x85, 0x16,
    ]);
    
    // Test 23: ORA 16-bit
    code.extend_from_slice(&[
        REP, 0x20,
        LDA_IMM, 0x0F, 0x00,
        ORA_IMM, 0xF0, 0x00,
        SEP, 0x20,
        0x85, 0x17,
    ]);
    
    // Test 24: EOR 16-bit
    code.extend_from_slice(&[
        REP, 0x20,
        LDA_IMM, 0xFF, 0x00,
        EOR_IMM, 0xAA, 0x00,
        SEP, 0x20,
        0x85, 0x18,
    ]);
    
    // =================================================================
    // SHIFT/ROTATE OPERATIONS (Tests 25-29)
    // =================================================================
    
    // Test 25: ASL A (arithmetic shift left)
    code.extend_from_slice(&[
        SEP, 0x30,
        CLC,
        LDA_IMM, 0x40,
        ASL_A,             // 0x40 << 1 = 0x80
        0x85, 0x19,
    ]);
    
    // Test 26: LSR A (logical shift right)
    code.extend_from_slice(&[
        CLC,
        LDA_IMM, 0x80,
        LSR_A,             // 0x80 >> 1 = 0x40
        0x85, 0x1A,
    ]);
    
    // Test 27: ROL A (rotate left through carry)
    code.extend_from_slice(&[
        SEC,               // Set carry
        LDA_IMM, 0x40,
        ROL_A,             // 0x40 << 1 | carry = 0x81
        0x85, 0x1B,
    ]);
    
    // Test 28: ROR A (rotate right through carry)
    code.extend_from_slice(&[
        SEC,
        LDA_IMM, 0x80,
        ROR_A,             // carry | 0x80 >> 1 = 0xC0
        0x85, 0x1C,
    ]);
    
    // Test 29: Multiple shifts
    code.extend_from_slice(&[
        CLC,
        LDA_IMM, 0x01,
        ASL_A,             // 0x01 -> 0x02
        ASL_A,             // 0x02 -> 0x04
        ASL_A,             // 0x04 -> 0x08
        0x85, 0x1D,
    ]);
    
    // =================================================================
    // TRANSFER OPERATIONS (Tests 30-34)
    // =================================================================
    
    // Test 30: TAX transfer
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x33,
        TAX,
        TXA,
        0x85, 0x1E,
    ]);
    
    // Test 31: TAY transfer
    code.extend_from_slice(&[
        LDA_IMM, 0x44,
        TAY,
        TYA,
        0x85, 0x1F,
    ]);
    
    // Test 32: TXA transfer
    code.extend_from_slice(&[
        LDX_IMM, 0x55,
        TXA,
        0x85, 0x20,
    ]);
    
    // Test 33: TYA transfer
    code.extend_from_slice(&[
        LDY_IMM, 0x66,
        TYA,
        0x85, 0x21,
    ]);
    
    // Test 34: TXY (65816 specific)
    code.extend_from_slice(&[
        LDX_IMM, 0x77,
        0x9B,              // TXY
        TYA,
        0x85, 0x22,
    ]);
    
    // =================================================================
    // STACK OPERATIONS (Tests 35-37)
    // =================================================================
    
    // Test 35: PHA/PLA
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x88,
        PHA,
        LDA_IMM, 0x00,
        PLA,               // Should restore 0x88
        0x85, 0x23,
    ]);
    
    // Test 36: PHP/PLP
    code.extend_from_slice(&[
        SEC,               // Set carry
        PHP,
        CLC,               // Clear carry
        PLP,               // Restore carry
        LDA_IMM, 0x00,
        0x69, 0x00,        // ADC #$00 (will be 1 if carry set)
        0x85, 0x24,
    ]);
    
    // Test 37: PHX/PLX (65816)
    code.extend_from_slice(&[
        SEP, 0x30,
        LDX_IMM, 0x99,
        0xDA,              // PHX
        LDX_IMM, 0x00,
        0xFA,              // PLX - restore 0x99
        TXA,
        0x85, 0x25,
    ]);
    
    // =================================================================
    // FLAG OPERATIONS (Tests 38-44)
    // =================================================================
    
    // Test 38: CLC
    code.extend_from_slice(&[
        SEP, 0x30,
        CLC,
        LDA_IMM, 0x10,
        ADC_IMM, 0x00,     // Should be 0x10
        0x85, 0x26,
    ]);
    
    // Test 39: SEC effect on ADC
    code.extend_from_slice(&[
        SEC,
        LDA_IMM, 0x10,
        ADC_IMM, 0x00,     // Should be 0x11 with carry
        0x85, 0x27,
    ]);
    
    // Test 40: CLV (clear overflow)
    code.extend_from_slice(&[
        CLV,
        LDA_IMM, 0x50,     // ADC that would set overflow
        ADC_IMM, 0x50,
        0x85, 0x28,
    ]);
    
    // Test 41: SEI/CLI (interrupt disable)
    code.extend_from_slice(&[
        SEI,
        CLI,
        LDA_IMM, 0x22,
        0x85, 0x29,
    ]);
    
    // Test 42: CLD (decimal mode)
    code.extend_from_slice(&[
        CLD,
        LDA_IMM, 0x23,
        0x85, 0x2A,
    ]);
    
    // Test 43: REP to clear multiple flags
    code.extend_from_slice(&[
        SEP, 0x30,
        SEC,
        REP, 0x01,         // Clear carry via REP
        LDA_IMM, 0x24,
        ADC_IMM, 0x00,
        0x85, 0x2B,
    ]);
    
    // Test 44: SEP to set multiple flags
    code.extend_from_slice(&[
        CLC,
        SEP, 0x01,         // Set carry via SEP
        LDA_IMM, 0x25,
        ADC_IMM, 0x00,     // Should add carry
        0x85, 0x2C,
    ]);
    
    // =================================================================
    // COMPARISON OPERATIONS (Tests 45-47)
    // =================================================================
    
    // Test 45: CMP sets flags correctly (equal)
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x50,
        CMP_IMM, 0x50,     // Equal, should set Z and C
        LDA_IMM, 0x50,     // Reload value for storage
        0x85, 0x2D,
    ]);
    
    // Test 46: CPX operation
    code.extend_from_slice(&[
        LDX_IMM, 0x60,
        CPX_IMM, 0x60,
        TXA,
        0x85, 0x2E,
    ]);
    
    // Test 47: CPY operation
    code.extend_from_slice(&[
        LDY_IMM, 0x70,
        CPY_IMM, 0x70,
        TYA,
        0x85, 0x2F,
    ]);
    
    // =================================================================
    // BRANCH OPERATIONS (Tests 48-51)
    // =================================================================
    
    // Test 48: BEQ (branch if equal)
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x80,
        CMP_IMM, 0x80,     // Sets Z flag
        BEQ, 0x02,         // Branch forward 2 bytes
        LDA_IMM, 0xFF,     // Skipped
        // Landed here
        0x85, 0x30,
    ]);
    
    // Test 49: BNE (branch if not equal)
    code.extend_from_slice(&[
        LDA_IMM, 0x81,
        CMP_IMM, 0x80,     // Not equal, Z clear
        BNE, 0x02,         // Branch forward
        LDA_IMM, 0xFF,     // Skipped
        // Landed here
        0x85, 0x31,
    ]);
    
    // Test 50: BCC (branch if carry clear)
    code.extend_from_slice(&[
        CLC,
        LDA_IMM, 0x82,
        BCC, 0x02,
        LDA_IMM, 0xFF,
        0x85, 0x32,
    ]);
    
    // Test 51: BCS (branch if carry set)
    code.extend_from_slice(&[
        SEC,
        LDA_IMM, 0x83,
        BCS, 0x02,
        LDA_IMM, 0xFF,
        0x85, 0x33,
    ]);
    
    // =================================================================
    // INDEXED ADDRESSING MODE TESTS (Tests 52-56)
    // =================================================================
    
    // Test 52: Store with direct page, load with direct page indexed
    // Store value at $50, load with X=0
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0xAB,
        0x85, 0x50,        // STA $50
        LDX_IMM, 0x00,
        0xB5, 0x50,        // LDA $50,X
        0x85, 0x34,
    ]);
    
    // Test 53: Direct page indexed with offset
    code.extend_from_slice(&[
        LDA_IMM, 0xCD,
        0x85, 0x55,        // STA $55
        LDX_IMM, 0x05,
        0xB5, 0x50,        // LDA $50,X (reads $55)
        0x85, 0x35,
    ]);
    
    // Test 54: Direct page X indexed for LDY
    code.extend_from_slice(&[
        LDA_IMM, 0xEF,
        0x85, 0x5A,        // STA $5A
        LDX_IMM, 0x0A,
        0xB4, 0x50,        // LDY $50,X (reads from $5A = $50+$0A)
        TYA,
        0x85, 0x36,
    ]);
    
    // Test 55: Absolute addressing (within bank 0)
    code.extend_from_slice(&[
        SEP, 0x30,
        LDA_IMM, 0x47,
        0x85, 0x60,        // STA $60
        0xA5, 0x60,        // LDA $60 (read it back)
        0x85, 0x37,
    ]);
    
    // Test 56: Complex sequence - multiple operations
    code.extend_from_slice(&[
        LDA_IMM, 0x58,
        0x85, 0x70,        // STA $70
        0xA5, 0x70,        // LDA $70
        0x85, 0x38,
    ]);
    
    // End with STP
    code.push(STP);
    
    build_test_rom(&code)
}

// =================================================================
// EXPECTED VALUES FOR COMPREHENSIVE TESTS
// =================================================================

/// Expected results for comprehensive test ROM
/// Returns (test_number, expected_value) pairs
pub fn get_comprehensive_test_expectations() -> Vec<(usize, u8)> {
    vec![
        (0, 0x34),   // Test 0: 16-bit load low byte
        (1, 0x56),   // Test 1: 16-bit X low byte
        (2, 0xCD),   // Test 2: 16-bit A low byte
        (3, 0x42),   // Test 3: LDA #$42
        (4, 0x55),   // Test 4: LDX #$55
        (5, 0xAA),   // Test 5: LDY #$AA
        (6, 0x34),   // Test 6: LDA 16-bit #$1234
        (7, 0x78),   // Test 7: LDX 16-bit #$7856
        (8, 0xBC),   // Test 8: LDY 16-bit #$9ABC
        (9, 0x30),   // Test 9: ADC $10 + $20
        (10, 0x31),  // Test 10: ADC with carry
        (11, 0x30),  // Test 11: SBC $50 - $20
        (12, 0x00),  // Test 12: ADC 16-bit low byte
        (13, 0x00),  // Test 13: SBC 16-bit low byte
        (14, 0x11),  // Test 14: INC $10 -> $11
        (15, 0x1F),  // Test 15: DEC $20 -> $1F
        (16, 0x31),  // Test 16: INX $30 -> $31
        (17, 0x3F),  // Test 17: DEX $40 -> $3F
        (18, 0x50),  // Test 18: INY/DEY
        (19, 0x0F),  // Test 19: AND
        (20, 0xFF),  // Test 20: ORA
        (21, 0x55),  // Test 21: EOR
        (22, 0x0F),  // Test 22: AND 16-bit
        (23, 0xFF),  // Test 23: ORA 16-bit
        (24, 0x55),  // Test 24: EOR 16-bit
        (25, 0x80),  // Test 25: ASL
        (26, 0x40),  // Test 26: LSR
        (27, 0x81),  // Test 27: ROL with carry
        (28, 0xC0),  // Test 28: ROR with carry
        (29, 0x08),  // Test 29: Multiple shifts
        (30, 0x33),  // Test 30: TAX
        (31, 0x44),  // Test 31: TAY
        (32, 0x55),  // Test 32: TXA
        (33, 0x66),  // Test 33: TYA
        (34, 0x77),  // Test 34: TXY
        (35, 0x88),  // Test 35: PHA/PLA
        (36, 0x01),  // Test 36: PHP/PLP carry
        (37, 0x99),  // Test 37: PHX/PLX
        (38, 0x10),  // Test 38: CLC
        (39, 0x11),  // Test 39: SEC
        (40, 0xA0),  // Test 40: CLV (0x50 + 0x50 = 0xA0)
        (41, 0x22),  // Test 41: SEI/CLI
        (42, 0x23),  // Test 42: CLD
        (43, 0x24),  // Test 43: REP clear carry
        (44, 0x26),  // Test 44: SEP set carry
        (45, 0x50),  // Test 45: CMP equal
        (46, 0x60),  // Test 46: CPX
        (47, 0x70),  // Test 47: CPY
        (48, 0x80),  // Test 48: BEQ
        (49, 0x81),  // Test 49: BNE
        (50, 0x82),  // Test 50: BCC
        (51, 0x83),  // Test 51: BCS
        (52, 0xAB),  // Test 52: Indexed load
        (53, 0xCD),  // Test 53: LDA with X offset
        (54, 0xEF),  // Test 54: LDY indexed
        (55, 0x47),  // Test 55: Absolute addressing
        (56, 0x58),  // Test 56: Complex sequence
    ]
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
    fn test_simple_wram_write() {
        let test_code = vec![
            SEP, 0x30,         // 8-bit mode
            LDA_IMM, 0x42,     // LDA #$42
            0x85, 0x00,        // STA $00 (direct page - writes to $7E0000)
            LDA_IMM, 0x00,     // Clear A
            0xA5, 0x00,        // LDA $00 (read it back)
            STP,               // Stop
        ];
        
        let rom = build_test_rom(&test_code);
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        // Execute all instructions
        for _ in 0..10 {
            if cpu.stopped {
                break;
            }
            cpu.step(&mut memory);
        }
        
        // Check that A contains 0x42
        assert_eq!(cpu.a & 0xFF, 0x42, "Expected A to contain 0x42 after read-back");
        
        // Also check memory directly
        let value = memory.read(0x7E0000);
        assert_eq!(value, 0x42, "Expected $7E0000 to contain 0x42");
    }
    
    #[test]
    fn test_metadata_array() {
        // Verify test metadata is accessible
        assert!(TEST_METADATA.len() > 0);
        assert_eq!(TEST_METADATA[0].name, "load_immediate");
        assert!(TEST_METADATA[0].description.len() > 0);
    }
    
    #[test]
    fn test_comprehensive_rom_execution() {
        let rom = build_comprehensive_test_rom();
        
        // Debug: check the ROM structure
        println!("ROM size: {}", rom.len());
        println!("Reset vector at $7FFC: {:02X} {:02X}", rom[0x7FFC], rom[0x7FFD]);
        println!("First bytes at ROM $0000: {:02X} {:02X} {:02X} {:02X}", 
                 rom[0], rom[1], rom[2], rom[3]);
        
        let (mut cpu, mut memory) = setup_cpu_with_rom(rom);
        
        println!("PC after reset: ${:04X}, PBR: ${:02X}", cpu.pc, cpu.pbr);
        
        // Run for many cycles to execute all tests
        let max_cycles = 50000;
        let mut cycles = 0;
        for i in 0..max_cycles {
            cpu.step(&mut memory);
            cycles = i + 1;
            if cpu.stopped {
                break;
            }
        }
        
        println!("Executed {} instructions, CPU stopped: {}", cycles, cpu.stopped);
        println!("Final PC: ${:04X}, PBR: ${:02X}", cpu.pc, cpu.pbr);
        
        // Check test results from direct page ($7E0000 + offset)
        let expectations = get_comprehensive_test_expectations();
        let mut passed = 0;
        let mut failed = 0;
        
        for (test_num, expected) in expectations.iter() {
            // Direct page $00-$FF maps to $7E0000-$7E00FF
            let addr = 0x7E0000 + (*test_num as u32);
            let actual = memory.read(addr);
            
            if actual == *expected {
                passed += 1;
            } else {
                failed += 1;
                if failed <= 10 {  // Only print first 10 failures
                    println!("Test {} failed: expected ${:02X}, got ${:02X}", 
                             test_num, expected, actual);
                }
            }
        }
        
        println!("Comprehensive tests: {} passed, {} failed out of {}", 
                 passed, failed, expectations.len());
        
        // Assert that most tests pass - this will expose implementation issues
        assert!(passed >= 40, "Expected at least 40 tests to pass, got {}. \
                This indicates CPU implementation issues that need to be fixed.", passed);
    }
}
