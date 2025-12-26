/// 65816 Addressing Modes Test
/// 
/// This test validates all 65816 addressing modes by running a comprehensive
/// assembly test program and verifying the results.
///
/// The assembly program tests:
/// - Immediate addressing (#$12, #$1234)
/// - Direct Page addressing ($12)
/// - Direct Page Indexed ($12,X and $12,Y)
/// - Absolute addressing ($1234)
/// - Absolute Indexed ($1234,X and $1234,Y)
/// - Absolute Long addressing ($123456)
/// - Direct Page Indirect (($12))
/// - Direct Page Indexed Indirect (($12,X))
/// - Direct Page Indirect Indexed (($12),Y)
/// - Absolute Indexed Indirect (($1234,X))
/// - Stack Relative ($12,S)
/// - Stack Relative Indirect Indexed (($12,S),Y)
/// - Different Direct Page values
/// - Page crossing conditions
/// - Bank wrapping conditions

use std::fs;
use znes_wasm::memory::Memory;
use znes_wasm::emulator::Emulator;

/// Test result structure matching the assembly program output
#[derive(Debug, Clone, Copy)]
struct TestResult {
    test_id: u8,
    expected: u8,
    actual: u8,
    pass_flag: u8,
}

impl TestResult {
    fn from_memory(memory: &mut Memory, index: usize) -> Self {
        let base_addr = 0x7E0100 + (index * 4);
        TestResult {
            test_id: memory.read(base_addr as u32),
            expected: memory.read((base_addr + 1) as u32),
            actual: memory.read((base_addr + 2) as u32),
            pass_flag: memory.read((base_addr + 3) as u32),
        }
    }

    fn passed(&self) -> bool {
        self.pass_flag == 1
    }
}

/// Load the test ROM into memory
fn load_test_rom() -> Result<Vec<u8>, String> {
    let rom_path = "tests/roms/addressing_modes_test.bin";
    
    match fs::read(rom_path) {
        Ok(rom_data) => Ok(rom_data),
        Err(_) => {
            // If binary doesn't exist, return an error with instructions
            Err(format!(
                "ROM file not found: {}\n\
                 Please assemble the test ROM first:\n\
                 1. Install a 65816 assembler (e.g., WLA-DX or ca65)\n\
                 2. Assemble: wla-65816 -o tests/roms/addressing_modes_test.bin tests/roms/addressing_modes_test.asm\n\
                 3. Run the test again",
                rom_path
            ))
        }
    }
}

/// Run the emulator until it reaches the infinite loop at the end
fn run_until_complete(emulator: &mut Emulator, max_cycles: u64) -> u64 {
    let mut pc_unchanged_count = 0;
    let mut last_pc = 0xFFFF;
    const PC_UNCHANGED_THRESHOLD: u32 = 10;
    
    loop {
        let cpu = emulator.cpu();
        let current_pc = cpu.pc;
        let current_cycles = cpu.cycles;
        
        // Check if stopped (PC unchanged for multiple steps)
        if current_pc == last_pc {
            pc_unchanged_count += 1;
            if pc_unchanged_count >= PC_UNCHANGED_THRESHOLD {
                break;
            }
        } else {
            pc_unchanged_count = 0;
            last_pc = current_pc;
        }
        
        // Check cycle limit
        if current_cycles >= max_cycles {
            break;
        }
        
        // Step CPU
        emulator.step();
    }
    
    emulator.cpu().cycles
}

/// Parse and display test results
fn parse_results(emulator: &mut Emulator) -> Vec<TestResult> {
    let mut results = Vec::new();
    let memory = emulator.memory_mut().expect("Memory not initialized");
    
    // Read up to 64 test results (256 bytes / 4 bytes per result)
    for i in 0..64 {
        let result = TestResult::from_memory(memory, i);
        
        // Stop when we hit an empty test (test_id == 0 and not initialized)
        if result.test_id == 0 && result.expected == 0 && result.actual == 0 {
            continue;
        }
        
        results.push(result);
    }
    
    results
}

/// Get the name of a test based on its ID
fn get_test_name(test_id: u8) -> &'static str {
    match test_id {
        1 => "Immediate 8-bit: LDA #$42",
        2 => "Immediate 16-bit: High byte of LDA #$1234",
        3 => "Immediate 16-bit: Low byte of LDA #$1234",
        4 => "Direct Page: Read from $12",
        5 => "Direct Page: Write to $12",
        6 => "Direct Page Indexed X: LDA $12,X (X=$0E)",
        7 => "Direct Page Indexed Y: LDA $12,Y (Y=$13)",
        8 => "Absolute: Read from $1234",
        9 => "Absolute: Write to $1234",
        10 => "Absolute Indexed X: LDA $1234,X (X=$0C)",
        11 => "Absolute Indexed Y: LDA $1234,Y (Y=$1C)",
        12 => "Absolute Long: Read from $7E2000",
        13 => "Absolute Long: Write to $7E2000",
        14 => "Direct Page Indirect: Read using ($30)",
        15 => "Direct Page Indirect: Write using ($30)",
        16 => "DP Indexed Indirect: Read using ($30,X) X=$10",
        17 => "DP Indexed Indirect: Write using ($30,X) X=$10",
        18 => "DP Indirect Indexed: Read using ($50),Y Y=$10",
        19 => "DP Indirect Indexed: Write using ($50),Y Y=$10",
        20 => "Absolute Indexed Indirect: Pointer read at $1250,X X=$10",
        21 => "Stack Relative: Read from $01,S",
        22 => "Stack Relative: Write to $01,S",
        23 => "Stack Relative Indirect Indexed: Read ($01,S),Y Y=$10",
        24 => "Stack Relative Indirect Indexed: Write ($01,S),Y Y=$10",
        25 => "Direct Page $2000: LDA $12",
        26 => "Direct Page $FF00: LDA $12",
        27 => "Page Crossing: Read at $12FF",
        28 => "Page Crossing: Read at $1300",
        29 => "Bank Wrapping: Read from $7EFFFF",
        30 => "Bank Wrapping: Read from $7E0000",
        _ => "Unknown test",
    }
}

#[test]
#[ignore] // Ignore by default since ROM needs to be assembled first
fn test_all_addressing_modes() {
    println!("\n=== 65816 Addressing Modes Test ===\n");
    
    // Load the test ROM
    let rom_data = match load_test_rom() {
        Ok(data) => {
            println!("✓ Test ROM loaded successfully");
            data
        }
        Err(e) => {
            println!("✗ Failed to load test ROM: {}", e);
            panic!("Cannot run test without ROM");
        }
    };
    
    // Create emulator and load ROM
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom_data)
        .expect("Failed to load ROM");
    let pc = emulator.cpu().pc;
    println!("✓ CPU reset to address ${:04X}", pc);
    
    // Run the test program
    println!("\nRunning test program...");
    let max_cycles = 100000;
    let cycles_used = run_until_complete(&mut emulator, max_cycles);
    println!("✓ Test program completed in {} cycles", cycles_used);
    
    // Parse results
    let results = parse_results(&mut emulator);
    println!("\n=== Test Results ({} tests) ===\n", results.len());
    
    let mut passed = 0;
    let mut failed = 0;
    
    for result in &results {
        let status = if result.passed() { "✓ PASS" } else { "✗ FAIL" };
        let test_name = get_test_name(result.test_id);
        
        println!(
            "{} [Test {:02}] {}: Expected ${:02X}, Got ${:02X}",
            status, result.test_id, test_name, result.expected, result.actual
        );
        
        if result.passed() {
            passed += 1;
        } else {
            failed += 1;
        }
    }
    
    // Print summary
    println!("\n=== Summary ===");
    println!("Total:  {} tests", results.len());
    println!("Passed: {} tests", passed);
    println!("Failed: {} tests", failed);
    
    if failed > 0 {
        println!("\n=== Failed Tests Details ===");
        for result in results.iter().filter(|r| !r.passed()) {
            println!(
                "Test {:02} ({}): Expected ${:02X}, Got ${:02X}",
                result.test_id,
                get_test_name(result.test_id),
                result.expected,
                result.actual
            );
        }
    }
    
    // Assert all tests passed
    assert_eq!(
        failed, 0,
        "\n{} addressing mode test(s) failed!",
        failed
    );
    
    println!("\n✓ All addressing mode tests passed!\n");
}

#[test]
fn test_immediate_addressing() {
    println!("\n=== Testing Immediate Addressing Mode ===\n");
    
    // Create a simple ROM with LDA #$42
    let mut rom = vec![0xFF; 0x8000];
    
    // Code at $8000
    rom[0x0000] = 0xA9; // LDA immediate
    rom[0x0001] = 0x42;
    rom[0x0002] = 0x5C; // JML (infinite loop)
    rom[0x0003] = 0x02;
    rom[0x0004] = 0x80;
    rom[0x0005] = 0x00;
    
    // Set reset vector to $8000
    rom[0x7FFC] = 0x00;
    rom[0x7FFD] = 0x80;
    
    // Create emulator and load ROM
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom)
        .expect("Failed to load ROM");
    
    // Execute one instruction
    emulator.step();
    
    // Verify result
    let cpu = emulator.cpu();
    assert_eq!(cpu.a & 0xFF, 0x42, "LDA #$42 should set A to $42");
    
    println!("✓ Immediate addressing test passed");
}

#[test]
fn test_direct_page_addressing() {
    println!("\n=== Testing Direct Page Addressing Mode ===\n");
    
    // Create a simple ROM
    let mut rom = vec![0xFF; 0x8000];
    
    // Code at $8000
    rom[0x0000] = 0xA5; // LDA direct page
    rom[0x0001] = 0x12;
    rom[0x0002] = 0x5C; // JML (infinite loop)
    rom[0x0003] = 0x02;
    rom[0x0004] = 0x80;
    rom[0x0005] = 0x00;
    
    // Set reset vector to $8000
    rom[0x7FFC] = 0x00;
    rom[0x7FFD] = 0x80;
    
    // Create emulator and load ROM
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom)
        .expect("Failed to load ROM");
    
    // Set up test data at DP+$12
    emulator.memory_mut()
        .expect("Memory not initialized")
        .write(0x7E0012, 0x55);
    
    // Execute one instruction
    emulator.step();
    
    // Verify result
    let cpu = emulator.cpu();
    assert_eq!(cpu.a & 0xFF, 0x55, "LDA $12 should load $55 from DP+$12");
    
    println!("✓ Direct Page addressing test passed");
}

#[test]
fn test_absolute_addressing() {
    println!("\n=== Testing Absolute Addressing Mode ===\n");
    
    // Create a simple ROM
    let mut rom = vec![0xFF; 0x8000];
    
    // Code at $8000
    rom[0x0000] = 0xAD; // LDA absolute
    rom[0x0001] = 0x34;
    rom[0x0002] = 0x12;
    rom[0x0003] = 0x5C; // JML (infinite loop)
    rom[0x0004] = 0x03;
    rom[0x0005] = 0x80;
    rom[0x0006] = 0x00;
    
    // Set reset vector to $8000
    rom[0x7FFC] = 0x00;
    rom[0x7FFD] = 0x80;
    
    // Create emulator and load ROM
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom)
        .expect("Failed to load ROM");
    
    // Set up test data at $7E:1234
    emulator.memory_mut()
        .expect("Memory not initialized")
        .write(0x7E1234, 0x66);
    
    // Execute one instruction
    emulator.step();
    
    // Verify result
    let cpu = emulator.cpu();
    assert_eq!(cpu.a & 0xFF, 0x66, "LDA $1234 should load $66 from $7E:1234");
    
    println!("✓ Absolute addressing test passed");
}

#[test]
fn test_indexed_addressing() {
    println!("\n=== Testing Indexed Addressing Modes ===\n");
    
    // Create a simple ROM
    let mut rom = vec![0xFF; 0x8000];
    
    // Code at $8000
    rom[0x0000] = 0xA2; // LDX immediate
    rom[0x0001] = 0x0C;
    rom[0x0002] = 0xBD; // LDA absolute,X
    rom[0x0003] = 0x34;
    rom[0x0004] = 0x12;
    rom[0x0005] = 0x5C; // JML (infinite loop)
    rom[0x0006] = 0x05;
    rom[0x0007] = 0x80;
    rom[0x0008] = 0x00;
    
    // Set reset vector to $8000
    rom[0x7FFC] = 0x00;
    rom[0x7FFD] = 0x80;
    
    // Create emulator and load ROM
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom)
        .expect("Failed to load ROM");
    
    // Set up test data at $7E:1240 ($1234 + $0C)
    emulator.memory_mut()
        .expect("Memory not initialized")
        .write(0x7E1240, 0x88);
    
    // Execute LDX #$0C
    emulator.step();
    
    // Execute LDA $1234,X
    emulator.step();
    
    // Verify result ($1234 + $0C = $1240)
    let cpu = emulator.cpu();
    assert_eq!(
        cpu.a & 0xFF,
        0x88,
        "LDA $1234,X should load $88 from $7E:1240"
    );
    
    println!("✓ Indexed addressing test passed");
}

#[cfg(test)]
mod addressing_mode_calculations {
    //! Documentation of address calculation for each mode
    
    /// Immediate: #$12
    /// - Value is in the instruction stream
    /// - No address calculation needed
    /// - Example: LDA #$42 loads the literal value $42
    #[test]
    fn document_immediate() {}
    
    /// Direct Page: $12
    /// - Address = Direct Page Register + $12
    /// - Example: If DP=$2000, then LDA $12 accesses $2012
    #[test]
    fn document_direct_page() {}
    
    /// Direct Page Indexed: $12,X
    /// - Address = (Direct Page Register + $12 + X) & $FFFF
    /// - Wraps within bank 0
    /// - Example: If DP=$2000 and X=$10, then LDA $12,X accesses $2022
    #[test]
    fn document_direct_page_indexed() {}
    
    /// Absolute: $1234
    /// - Address = Data Bank Register : $1234
    /// - Example: If DBR=$7E, then LDA $1234 accesses $7E:1234
    #[test]
    fn document_absolute() {}
    
    /// Absolute Indexed: $1234,X
    /// - Address = Data Bank Register : ($1234 + X) & $FFFF
    /// - Wraps within the data bank
    /// - Example: If DBR=$7E and X=$10, then LDA $1234,X accesses $7E:1244
    #[test]
    fn document_absolute_indexed() {}
    
    /// Absolute Long: $123456
    /// - Address = $123456 (full 24-bit address)
    /// - No bank register involved
    /// - Example: LDA $7E2000 accesses $7E:2000
    #[test]
    fn document_absolute_long() {}
    
    /// Direct Page Indirect: ($12)
    /// - Pointer Address = Direct Page Register + $12
    /// - Final Address = Data Bank Register : [Pointer Address]
    /// - Example: If DP=$0000 and [$0012]=$2100, then LDA ($12) accesses DBR:2100
    #[test]
    fn document_dp_indirect() {}
    
    /// Direct Page Indexed Indirect: ($12,X)
    /// - Pointer Address = (Direct Page Register + $12 + X) & $FFFF
    /// - Final Address = Data Bank Register : [Pointer Address]
    /// - Example: If DP=$0000, X=$10, and [$0022]=$2200, then LDA ($12,X) accesses DBR:2200
    #[test]
    fn document_dp_indexed_indirect() {}
    
    /// Direct Page Indirect Indexed: ($12),Y
    /// - Pointer Address = Direct Page Register + $12
    /// - Final Address = Data Bank Register : ([Pointer Address] + Y) & $FFFF
    /// - Example: If DP=$0000, Y=$10, and [$0012]=$2300, then LDA ($12),Y accesses DBR:2310
    #[test]
    fn document_dp_indirect_indexed() {}
    
    /// Absolute Indexed Indirect: ($1234,X)
    /// - Used primarily with JMP/JSR
    /// - Pointer Address = Program Bank : ($1234 + X) & $FFFF
    /// - Final Address = [Pointer Address] (16 or 24-bit depending on instruction)
    #[test]
    fn document_absolute_indexed_indirect() {}
    
    /// Stack Relative: $12,S
    /// - Address = Stack Pointer + $12
    /// - Always accesses bank 0
    /// - Example: If SP=$01FF, then LDA $12,S accesses $00:0211
    #[test]
    fn document_stack_relative() {}
    
    /// Stack Relative Indirect Indexed: ($12,S),Y
    /// - Pointer Address = Stack Pointer + $12
    /// - Final Address = Data Bank Register : ([Pointer Address] + Y) & $FFFF
    /// - Example: If SP=$01FF, Y=$10, and [$0211]=$2400, then LDA ($12,S),Y accesses DBR:2410
    #[test]
    fn document_stack_relative_indirect_indexed() {}
}
