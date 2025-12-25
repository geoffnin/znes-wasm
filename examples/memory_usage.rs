// Example usage of the SNES memory system and cartridge loading
//
// This file demonstrates how to use the implemented SNES memory system
// and ROM cartridge loading functionality.

use znes_wasm::cartridge::{Cartridge, MappingMode, Region};
use znes_wasm::memory::Memory;

fn main() {
    // Example 1: Loading a ROM file
    println!("=== Example 1: Loading a ROM ===");
    
    // In a real application, you would load the ROM from a file
    let rom_data = create_example_rom();
    
    match Cartridge::from_rom(rom_data) {
        Ok(cartridge) => {
            println!("ROM loaded successfully!");
            println!("  Title: {}", cartridge.title());
            println!("  Region: {:?}", cartridge.region());
            println!("  Mapping Mode: {:?}", cartridge.mapping_mode());
            println!("  ROM Size: {} KB", cartridge.rom_size() / 1024);
            println!("  SRAM Size: {} KB", cartridge.sram_size() / 1024);
            
            // Example 2: Creating memory system
            println!("\n=== Example 2: Creating Memory System ===");
            let mut memory = Memory::new(&cartridge);
            println!("Memory system initialized with {} mapping", 
                     match cartridge.mapping_mode() {
                         MappingMode::LoRom => "LoROM",
                         MappingMode::HiRom => "HiROM",
                         MappingMode::ExHiRom => "ExHiROM",
                     });
            
            // Example 3: Writing and reading WRAM
            println!("\n=== Example 3: WRAM Access ===");
            memory.write(0x7E0000, 0x42);
            memory.write(0x7E0001, 0x43);
            println!("Wrote $42 to $7E0000");
            println!("Read back: ${:02X}", memory.read(0x7E0000));
            
            // Example 4: 16-bit word access
            println!("\n=== Example 4: 16-bit Word Access ===");
            memory.write_word(0x7E0100, 0x1234);
            println!("Wrote $1234 to $7E0100");
            println!("Read back: ${:04X}", memory.read_word(0x7E0100));
            
            // Example 5: WRAM mirroring
            println!("\n=== Example 5: WRAM Mirroring ===");
            memory.write(0x000100, 0xAB);
            println!("Wrote $AB to $000100 (bank $00)");
            println!("Read from $800100 (bank $80): ${:02X}", memory.read(0x800100));
            println!("Mirrors work correctly!");
            
            // Example 6: ROM reading
            println!("\n=== Example 6: ROM Access ===");
            let rom_byte = memory.read(0x008000);
            println!("First byte of ROM: ${:02X}", rom_byte);
            
            // Example 7: SRAM access (if available)
            if cartridge.sram_size() > 0 {
                println!("\n=== Example 7: SRAM Access ===");
                
                // Write to SRAM (address depends on mapping mode)
                let sram_addr = match cartridge.mapping_mode() {
                    MappingMode::LoRom => 0x708000, // Bank $70, offset $8000
                    MappingMode::HiRom => 0x006000, // Bank $00, offset $6000
                    MappingMode::ExHiRom => 0x006000,
                };
                
                memory.write(sram_addr, 0x55);
                println!("Wrote $55 to SRAM at ${:06X}", sram_addr);
                println!("Read back: ${:02X}", memory.read(sram_addr));
                
                // Save SRAM to "file"
                let sram_data = memory.sram();
                println!("SRAM size: {} bytes", sram_data.len());
            }
            
            // Example 8: Demonstrating memory regions
            println!("\n=== Example 8: Memory Regions Summary ===");
            println!("WRAM:  Banks $7E-$7F, 128KB total");
            println!("WRAM Mirror: Banks $00-$3F and $80-$BF at $0000-$1FFF");
            
            match cartridge.mapping_mode() {
                MappingMode::LoRom => {
                    println!("ROM:   Banks $00-$7D at $8000-$FFFF (LoROM)");
                    println!("SRAM:  Banks $70-$7D at $8000-$FFFF");
                }
                MappingMode::HiRom => {
                    println!("ROM:   Banks $C0-$FF at $0000-$FFFF (HiROM)");
                    println!("SRAM:  Banks $00-$3F at $6000-$7FFF");
                }
                MappingMode::ExHiRom => {
                    println!("ROM:   Extended HiROM mapping (up to 8MB)");
                    println!("SRAM:  Banks $00-$3F at $6000-$7FFF");
                }
            }
        }
        Err(e) => {
            eprintln!("Error loading ROM: {}", e);
        }
    }
}

// Create a minimal example ROM for demonstration
fn create_example_rom() -> Vec<u8> {
    let mut rom = vec![0; 0x8000]; // 32KB test ROM
    
    // Write header at LoROM location ($7FC0)
    let header_offset = 0x7FC0;
    
    // Title (21 bytes, padded with spaces)
    let title = b"EXAMPLE ROM          ";
    rom[header_offset..header_offset + 21].copy_from_slice(title);
    
    // Mapping mode: LoROM, Fast ($20)
    rom[header_offset + 0x15] = 0x20;
    
    // Cartridge type: ROM+RAM+Battery ($02)
    rom[header_offset + 0x16] = 0x02;
    
    // ROM size: 32KB = 2^15 = $08
    rom[header_offset + 0x17] = 0x08;
    
    // SRAM size: 8KB = 2^13 = $03
    rom[header_offset + 0x18] = 0x03;
    
    // Region: North America ($01)
    rom[header_offset + 0x19] = 0x01;
    
    // Checksum complement and checksum
    rom[header_offset + 0x1C] = 0x00;
    rom[header_offset + 0x1D] = 0x00;
    rom[header_offset + 0x1E] = 0xFF;
    rom[header_offset + 0x1F] = 0xFF;
    
    // Put some example data at the start of ROM
    rom[0] = 0x78; // SEI instruction
    rom[1] = 0x18; // CLC instruction
    rom[2] = 0xFB; // XCE instruction
    
    rom
}

// Example: Loading from a real file
#[allow(dead_code)]
fn load_rom_from_file(path: &str) -> Result<Cartridge, String> {
    use std::fs;
    
    let rom_data = fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    Cartridge::from_rom(rom_data)
}

// Example: Saving SRAM to file
#[allow(dead_code)]
fn save_sram(memory: &Memory, path: &str) -> Result<(), String> {
    use std::fs;
    
    fs::write(path, memory.sram())
        .map_err(|e| format!("Failed to write SRAM: {}", e))
}

// Example: Loading SRAM from file
#[allow(dead_code)]
fn load_sram(memory: &mut Memory, path: &str) -> Result<(), String> {
    use std::fs;
    
    let sram_data = fs::read(path)
        .map_err(|e| format!("Failed to read SRAM: {}", e))?;
    
    memory.load_sram(&sram_data);
    Ok(())
}

#[cfg(test)]
mod example_tests {
    use super::*;
    
    #[test]
    fn test_example_rom_creation() {
        let rom = create_example_rom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        
        assert_eq!(cartridge.title(), "EXAMPLE ROM");
        assert_eq!(cartridge.region(), Region::NorthAmerica);
        assert_eq!(cartridge.mapping_mode(), MappingMode::LoRom);
    }
    
    #[test]
    fn test_memory_operations() {
        let rom = create_example_rom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test write and read
        memory.write(0x7E0000, 0x42);
        assert_eq!(memory.read(0x7E0000), 0x42);
        
        // Test mirroring
        memory.write(0x000100, 0xAB);
        assert_eq!(memory.read(0x800100), 0xAB);
    }
}
