use crate::cartridge::{Cartridge, MappingMode};

/// SNES Memory System
/// 
/// The SNES uses a 24-bit address space (16MB) organized into 256 banks of 64KB each.
/// Banks are accessed through bank switching, and memory mapping depends on the cartridge type.
pub struct Memory {
    /// Work RAM - 128KB (mirrored in multiple locations)
    wram: Box<[u8; 0x20000]>, // 128KB
    
    /// Save RAM - Variable size depending on cartridge (typically 0-32KB)
    sram: Vec<u8>,
    
    /// ROM data from cartridge
    rom: Vec<u8>,
    
    /// Current mapping mode (determines address translation)
    mapping_mode: MappingMode,
    
    /// Lookup table for fast address translation
    /// Maps each 8KB page to its memory type and offset
    read_map: [MemoryRegion; 2048], // 16MB / 8KB = 2048 pages
    write_map: [MemoryRegion; 2048],
}

/// Represents a mapped memory region
#[derive(Copy, Clone, Debug)]
struct MemoryRegion {
    region_type: RegionType,
    offset: usize,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum RegionType {
    None,       // Unmapped/open bus
    Wram,       // Work RAM
    Sram,       // Save RAM
    Rom,        // ROM data
}

impl Default for MemoryRegion {
    fn default() -> Self {
        MemoryRegion {
            region_type: RegionType::None,
            offset: 0,
        }
    }
}

impl Memory {
    /// Create a new Memory system with a loaded cartridge
    pub fn new(cartridge: &Cartridge) -> Self {
        let mut memory = Memory {
            wram: Box::new([0; 0x20000]),
            sram: vec![0; cartridge.sram_size()],
            rom: cartridge.rom_data().to_vec(),
            mapping_mode: cartridge.mapping_mode(),
            read_map: [MemoryRegion::default(); 2048],
            write_map: [MemoryRegion::default(); 2048],
        };
        
        memory.initialize_memory_map();
        memory
    }
    
    /// Initialize the memory mapping lookup tables based on cartridge type
    fn initialize_memory_map(&mut self) {
        match self.mapping_mode {
            MappingMode::LoRom => self.initialize_lorom_map(),
            MappingMode::HiRom => self.initialize_hirom_map(),
            MappingMode::ExHiRom => self.initialize_exhirom_map(),
        }
    }
    
    /// LoROM mapping: ROM is mapped to banks $00-$7D and $80-$FD in upper half ($8000-$FFFF)
    /// Lower half contains WRAM mirrors and I/O
    fn initialize_lorom_map(&mut self) {
        // Map each 8KB page
        for page in 0..2048 {
            let bank = page / 8;
            let page_in_bank = page % 8;
            let _addr = (bank << 16) | (page_in_bank << 13);
            
            // Banks $00-$3F and $80-$BF (mirrored)
            if (bank <= 0x3F) || (bank >= 0x80 && bank <= 0xBF) {
                let effective_bank = bank & 0x3F;
                
                match page_in_bank {
                    // $0000-$1FFF: WRAM (first 8KB)
                    0 => {
                        self.read_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                        self.write_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                    }
                    // $2000-$3FFF: I/O registers (not implemented yet)
                    1 => {
                        // Skip for now
                    }
                    // $4000-$5FFF: I/O registers
                    2 => {
                        // Skip for now
                    }
                    // $6000-$7FFF: Expansion (not used in LoROM)
                    3 => {
                        // Skip for now
                    }
                    // $8000-$FFFF: ROM mapped here
                    4..=7 => {
                        let rom_bank_offset = (effective_bank as usize) * 0x8000;
                        let page_offset = ((page_in_bank - 4) as usize) * 0x2000;
                        let rom_offset = rom_bank_offset + page_offset;
                        
                        if rom_offset < self.rom.len() {
                            self.read_map[page] = MemoryRegion {
                                region_type: RegionType::Rom,
                                offset: rom_offset,
                            };
                        }
                    }
                    _ => {}
                }
            }
            // Banks $40-$6F and $C0-$EF: Extended ROM area
            else if (bank >= 0x40 && bank <= 0x6F) || (bank >= 0xC0 && bank <= 0xEF) {
                let effective_bank = bank & 0x3F;
                
                // Only upper half is ROM in these banks
                if page_in_bank >= 4 {
                    let rom_bank_offset = ((effective_bank + 0x40) as usize) * 0x8000;
                    let page_offset = ((page_in_bank - 4) as usize) * 0x2000;
                    let rom_offset = rom_bank_offset + page_offset;
                    
                    if rom_offset < self.rom.len() {
                        self.read_map[page] = MemoryRegion {
                            region_type: RegionType::Rom,
                            offset: rom_offset,
                        };
                    }
                }
            }
            // Banks $70-$7D and $F0-$FD: SRAM area (mirrored)
            else if (bank >= 0x70 && bank <= 0x7D) || (bank >= 0xF0 && bank <= 0xFD) {
                if page_in_bank >= 4 && !self.sram.is_empty() {
                    let sram_offset = ((bank & 0x0F) as usize) * 0x8000 + ((page_in_bank - 4) as usize) * 0x2000;
                    let sram_offset = sram_offset % self.sram.len();
                    
                    self.read_map[page] = MemoryRegion {
                        region_type: RegionType::Sram,
                        offset: sram_offset,
                    };
                    self.write_map[page] = MemoryRegion {
                        region_type: RegionType::Sram,
                        offset: sram_offset,
                    };
                }
            }
            // Banks $7E-$7F: Extended WRAM
            else if bank == 0x7E || bank == 0x7F {
                let wram_offset = ((bank - 0x7E) as usize) * 0x10000 + (page_in_bank as usize) * 0x2000;
                self.read_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
                self.write_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
            }
        }
    }
    
    /// HiROM mapping: ROM is mapped linearly starting at bank $C0
    /// Banks $00-$3F and $80-$BF have ROM in upper half
    fn initialize_hirom_map(&mut self) {
        for page in 0..2048 {
            let bank = page / 8;
            let page_in_bank = page % 8;
            
            // Banks $00-$3F and $80-$BF
            if (bank <= 0x3F) || (bank >= 0x80 && bank <= 0xBF) {
                let effective_bank = bank & 0x3F;
                
                match page_in_bank {
                    // $0000-$1FFF: WRAM (first 8KB)
                    0 => {
                        self.read_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                        self.write_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                    }
                    // $2000-$5FFF: I/O and expansion
                    1..=2 => {
                        // Skip for now
                    }
                    // $6000-$7FFF: SRAM (if present)
                    3 => {
                        if !self.sram.is_empty() {
                            let sram_offset = (effective_bank as usize) * 0x2000;
                            let sram_offset = sram_offset % self.sram.len();
                            
                            self.read_map[page] = MemoryRegion {
                                region_type: RegionType::Sram,
                                offset: sram_offset,
                            };
                            self.write_map[page] = MemoryRegion {
                                region_type: RegionType::Sram,
                                offset: sram_offset,
                            };
                        }
                    }
                    // $8000-$FFFF: ROM
                    4..=7 => {
                        let rom_offset = (effective_bank as usize) * 0x10000 + ((page_in_bank - 4) as usize) * 0x2000;
                        
                        if rom_offset < self.rom.len() {
                            self.read_map[page] = MemoryRegion {
                                region_type: RegionType::Rom,
                                offset: rom_offset,
                            };
                        }
                    }
                    _ => {}
                }
            }
            // Banks $40-$7D and $C0-$FF: Linear ROM mapping
            else if (bank >= 0x40 && bank <= 0x7D) || (bank >= 0xC0) {
                let rom_bank = if bank >= 0xC0 { bank - 0x80 } else { bank };
                let rom_offset = (rom_bank as usize) * 0x10000 + (page_in_bank as usize) * 0x2000;
                
                if rom_offset < self.rom.len() {
                    self.read_map[page] = MemoryRegion {
                        region_type: RegionType::Rom,
                        offset: rom_offset,
                    };
                }
            }
            // Banks $7E-$7F: Extended WRAM
            else if bank == 0x7E || bank == 0x7F {
                let wram_offset = ((bank - 0x7E) as usize) * 0x10000 + (page_in_bank as usize) * 0x2000;
                self.read_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
                self.write_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
            }
        }
    }
    
    /// ExHiROM mapping: Extended HiROM for larger ROMs (up to 8MB)
    fn initialize_exhirom_map(&mut self) {
        // ExHiROM is similar to HiROM but with extended addressing
        // Banks $00-$3F/$80-$BF: Similar to HiROM
        // Banks $40-$7D/$C0-$FF: Extended ROM area with special mapping
        
        for page in 0..2048 {
            let bank = page / 8;
            let page_in_bank = page % 8;
            
            if (bank <= 0x3F) || (bank >= 0x80 && bank <= 0xBF) {
                let effective_bank = bank & 0x3F;
                
                match page_in_bank {
                    0 => {
                        self.read_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                        self.write_map[page] = MemoryRegion {
                            region_type: RegionType::Wram,
                            offset: 0,
                        };
                    }
                    3 => {
                        if !self.sram.is_empty() {
                            let sram_offset = (effective_bank as usize) * 0x2000;
                            let sram_offset = sram_offset % self.sram.len();
                            
                            self.read_map[page] = MemoryRegion {
                                region_type: RegionType::Sram,
                                offset: sram_offset,
                            };
                            self.write_map[page] = MemoryRegion {
                                region_type: RegionType::Sram,
                                offset: sram_offset,
                            };
                        }
                    }
                    4..=7 => {
                        let rom_offset = ((effective_bank + 0x40) as usize) * 0x10000 + ((page_in_bank - 4) as usize) * 0x2000;
                        
                        if rom_offset < self.rom.len() {
                            self.read_map[page] = MemoryRegion {
                                region_type: RegionType::Rom,
                                offset: rom_offset,
                            };
                        }
                    }
                    _ => {}
                }
            } else if (bank >= 0x40 && bank <= 0x7D) || (bank >= 0xC0) {
                let rom_offset = (bank as usize) * 0x10000 + (page_in_bank as usize) * 0x2000;
                
                if rom_offset < self.rom.len() {
                    self.read_map[page] = MemoryRegion {
                        region_type: RegionType::Rom,
                        offset: rom_offset,
                    };
                }
            } else if bank == 0x7E || bank == 0x7F {
                let wram_offset = ((bank - 0x7E) as usize) * 0x10000 + (page_in_bank as usize) * 0x2000;
                self.read_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
                self.write_map[page] = MemoryRegion {
                    region_type: RegionType::Wram,
                    offset: wram_offset,
                };
            }
        }
    }
    
    /// Read a byte from memory using 24-bit address
    pub fn read(&self, addr: u32) -> u8 {
        let page = ((addr >> 13) & 0x7FF) as usize; // Get 8KB page number
        let offset_in_page = (addr & 0x1FFF) as usize;
        
        let region = self.read_map[page];
        
        match region.region_type {
            RegionType::Wram => {
                let addr = (region.offset + offset_in_page) % self.wram.len();
                self.wram[addr]
            }
            RegionType::Sram => {
                let addr = (region.offset + offset_in_page) % self.sram.len();
                self.sram[addr]
            }
            RegionType::Rom => {
                let addr = region.offset + offset_in_page;
                if addr < self.rom.len() {
                    self.rom[addr]
                } else {
                    0xFF // Open bus
                }
            }
            RegionType::None => 0xFF, // Open bus
        }
    }
    
    /// Write a byte to memory using 24-bit address
    pub fn write(&mut self, addr: u32, value: u8) {
        let page = ((addr >> 13) & 0x7FF) as usize;
        let offset_in_page = (addr & 0x1FFF) as usize;
        
        let region = self.write_map[page];
        
        match region.region_type {
            RegionType::Wram => {
                let addr = (region.offset + offset_in_page) % self.wram.len();
                self.wram[addr] = value;
            }
            RegionType::Sram => {
                let addr = (region.offset + offset_in_page) % self.sram.len();
                self.sram[addr] = value;
            }
            RegionType::Rom | RegionType::None => {
                // ROM and unmapped areas are not writable
            }
        }
    }
    
    /// Read a 16-bit word from memory (little-endian)
    pub fn read_word(&self, addr: u32) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }
    
    /// Write a 16-bit word to memory (little-endian)
    pub fn write_word(&mut self, addr: u32, value: u16) {
        self.write(addr, (value & 0xFF) as u8);
        self.write(addr.wrapping_add(1), (value >> 8) as u8);
    }
    
    /// Get SRAM data for saving
    pub fn sram(&self) -> &[u8] {
        &self.sram
    }
    
    /// Load SRAM data
    pub fn load_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.sram.len());
        self.sram[..len].copy_from_slice(&data[..len]);
    }
    
    /// Reset WRAM to power-on state
    pub fn reset(&mut self) {
        // WRAM is not cleared on reset, but we'll provide this method
        // for a full power cycle
        for byte in self.wram.iter_mut() {
            *byte = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;
    
    fn create_test_rom_lorom() -> Vec<u8> {
        let mut rom = vec![0; 0x8000]; // 32KB test ROM
        
        // Write header at offset $7FC0
        let header_offset = 0x7FC0;
        
        // ROM title (exactly 21 bytes)
        let title = b"TEST ROM             "; // 21 bytes
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        
        // Mapping mode: LoROM ($20)
        rom[header_offset + 0x15] = 0x20;
        
        // Cartridge type
        rom[header_offset + 0x16] = 0x00;
        
        // ROM size (32KB = $08)
        rom[header_offset + 0x17] = 0x08;
        
        // SRAM size (8KB = $03)
        rom[header_offset + 0x18] = 0x03;
        
        // Region (USA = $01)
        rom[header_offset + 0x19] = 0x01;
        
        // Checksum complement and checksum
        rom[header_offset + 0x1C] = 0xFF;
        rom[header_offset + 0x1D] = 0xFF;
        rom[header_offset + 0x1E] = 0x00;
        rom[header_offset + 0x1F] = 0x00;
        
        rom
    }
    
    #[test]
    fn test_memory_wram_access() {
        let rom = create_test_rom_lorom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test WRAM write and read at $7E0000
        memory.write(0x7E0000, 0x42);
        assert_eq!(memory.read(0x7E0000), 0x42);
        
        // Test WRAM mirror at $000000
        memory.write(0x000100, 0xAB);
        assert_eq!(memory.read(0x000100), 0xAB);
        
        // Test WRAM mirror in bank $80
        memory.write(0x800200, 0xCD);
        assert_eq!(memory.read(0x800200), 0xCD);
    }
    
    #[test]
    fn test_memory_rom_access() {
        let mut rom = create_test_rom_lorom();
        rom[0] = 0x12;
        rom[0x100] = 0x34;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // Test ROM read in LoROM mapping (bank $00, offset $8000+)
        assert_eq!(memory.read(0x008000), 0x12);
        assert_eq!(memory.read(0x008100), 0x34);
        
        // Test ROM mirror in bank $80
        assert_eq!(memory.read(0x808000), 0x12);
    }
    
    #[test]
    fn test_memory_word_access() {
        let rom = create_test_rom_lorom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test 16-bit write and read
        memory.write_word(0x7E0000, 0x1234);
        assert_eq!(memory.read_word(0x7E0000), 0x1234);
        assert_eq!(memory.read(0x7E0000), 0x34); // Little-endian
        assert_eq!(memory.read(0x7E0001), 0x12);
    }
    
    #[test]
    fn test_memory_sram_access() {
        let rom = create_test_rom_lorom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // LoROM SRAM is in bank $70-$7D at $8000-$FFFF (upper half)
        // For this test, write to bank $70, address $8000
        memory.write(0x708000, 0xAA);
        assert_eq!(memory.read(0x708000), 0xAA);
        
        // Test SRAM save/load
        let sram_data = memory.sram();
        assert_eq!(sram_data.len(), 8192); // 8KB as specified in header
    }
    
    // Helper function to create a test ROM with ExHiROM mapping
    fn create_test_rom_exhirom() -> Vec<u8> {
        // Create a 16MB ROM to properly test all ExHiROM mapping including banks $C0+
        // This allows testing the full range of ExHiROM addressing
        const EXHIROM_TEST_ROM_SIZE: usize = 0x1000000; // 16MB
        let mut rom = vec![0; EXHIROM_TEST_ROM_SIZE];
        
        // Write header at offset $FFC0 (HiROM/ExHiROM header location)
        let header_offset = 0xFFC0;
        
        // ROM title (exactly 21 bytes)
        let title = b"TEST EXHIROM ROM     "; // 21 bytes
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        
        // Mapping mode: ExHiROM ($25)
        rom[header_offset + 0x15] = 0x25;
        
        // Cartridge type
        rom[header_offset + 0x16] = 0x00;
        
        // ROM size: Using $0C (8MB) as the declared size in header
        // Note: The actual ROM buffer is 16MB for testing purposes, but we declare 8MB
        // in the header as $0D (16MB) is not a standard SNES ROM size
        rom[header_offset + 0x17] = 0x0C;
        
        // SRAM size (8KB = $03)
        rom[header_offset + 0x18] = 0x03;
        
        // Region (USA = $01)
        rom[header_offset + 0x19] = 0x01;
        
        // Checksum complement and checksum
        rom[header_offset + 0x1C] = 0xFF;
        rom[header_offset + 0x1D] = 0xFF;
        rom[header_offset + 0x1E] = 0x00;
        rom[header_offset + 0x1F] = 0x00;
        
        rom
    }
    
    #[test]
    fn test_exhirom_wram_access() {
        let rom = create_test_rom_exhirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Verify mapping mode is ExHiROM
        assert_eq!(cartridge.mapping_mode(), MappingMode::ExHiRom);
        
        // Test WRAM access at banks $7E-$7F (128KB WRAM)
        memory.write(0x7E0000, 0x42);
        assert_eq!(memory.read(0x7E0000), 0x42);
        
        memory.write(0x7F0000, 0x43);
        assert_eq!(memory.read(0x7F0000), 0x43);
        
        // Test WRAM mirror at $00:0000-$1FFF
        memory.write(0x000100, 0xAB);
        assert_eq!(memory.read(0x000100), 0xAB);
        
        // Test WRAM mirror in bank $80 (mirrored from $00)
        memory.write(0x800200, 0xCD);
        assert_eq!(memory.read(0x800200), 0xCD);
        
        // Test WRAM mirror in bank $20
        memory.write(0x200300, 0xEF);
        assert_eq!(memory.read(0x200300), 0xEF);
    }
    
    #[test]
    fn test_exhirom_rom_access_standard_banks() {
        let mut rom = create_test_rom_exhirom();
        
        // In ExHiROM, banks $00-$3F at $8000-$FFFF map to ROM offset (effective_bank + 0x40) * 0x10000 + page_offset
        // Bank $00:8000 -> ROM offset 0x400000 + 0x0000 = 0x400000
        rom[0x400000] = 0x12;
        rom[0x400100] = 0x34;
        
        // Bank $01:8000 -> ROM offset (0x01 + 0x40) * 0x10000 + 0x0000 = 0x410000
        rom[0x410000] = 0x56;
        
        // Bank $01:C000 -> ROM offset 0x410000 + 0x4000 = 0x414000
        rom[0x414000] = 0x78;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // Test ROM read in banks $00-$3F at $8000-$FFFF
        assert_eq!(memory.read(0x008000), 0x12); // Bank $00:8000
        assert_eq!(memory.read(0x008100), 0x34); // Bank $00:8100
        
        // Test bank $01
        assert_eq!(memory.read(0x018000), 0x56); // Bank $01:8000
        assert_eq!(memory.read(0x01C000), 0x78); // Bank $01:C000
        
        // Test ROM mirror in banks $80-$BF (should mirror $00-$3F)
        assert_eq!(memory.read(0x808000), 0x12); // Bank $80:8000 (mirrors $00:8000)
        assert_eq!(memory.read(0x818000), 0x56); // Bank $81:8000 (mirrors $01:8000)
    }
    
    #[test]
    fn test_exhirom_rom_access_extended_banks() {
        let mut rom = create_test_rom_exhirom();
        
        // Banks $40-$7D map linearly to ROM offset bank * 0x10000
        // Bank $40:0000 -> ROM offset 0x400000 (within our 16MB test ROM)
        rom[0x400000] = 0xAA; // Bank $40:0000
        rom[0x500000] = 0xBB; // Bank $50:0000
        rom[0x600000] = 0xCC; // Bank $60:0000
        
        // Banks $C0+ also map linearly to ROM
        // Bank $C0 should map to ROM offset 0xC0 * 0x10000 = 0xC00000
        rom[0xC00000] = 0xDD;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // Test ROM access in banks $40-$7D (linear mapping)
        assert_eq!(memory.read(0x400000), 0xAA); // Bank $40:0000
        assert_eq!(memory.read(0x500000), 0xBB); // Bank $50:0000
        assert_eq!(memory.read(0x600000), 0xCC); // Bank $60:0000
        
        // Test ROM access in banks $C0+
        assert_eq!(memory.read(0xC00000), 0xDD); // Bank $C0:0000
    }
    
    #[test]
    fn test_exhirom_sram_access() {
        let rom = create_test_rom_exhirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // ExHiROM SRAM is in banks $00-$3F and $80-$BF at $6000-$7FFF
        // Test SRAM write and read in bank $00
        memory.write(0x006000, 0xAA);
        assert_eq!(memory.read(0x006000), 0xAA);
        
        // Test SRAM in bank $01
        memory.write(0x016000, 0xBB);
        assert_eq!(memory.read(0x016000), 0xBB);
        
        // Test SRAM mirror in bank $80
        memory.write(0x806000, 0xCC);
        assert_eq!(memory.read(0x806000), 0xCC);
        
        // Verify SRAM size
        let sram_data = memory.sram();
        assert_eq!(sram_data.len(), 8192); // 8KB as specified in header
    }
    
    #[test]
    fn test_exhirom_address_translation() {
        let mut rom = create_test_rom_exhirom();
        
        // Place distinctive data at specific ROM locations to verify address translation
        // In ExHiROM, banks $00-$3F at $8000-$FFFF map to ROM at (effective_bank + 0x40) * 0x10000 + offset
        
        // Bank $00:8000 -> ROM offset 0x400000
        rom[0x400000] = 0x11;
        
        // Bank $00:C000 -> ROM offset 0x400000 + 0x4000 = 0x404000
        rom[0x404000] = 0x22;
        
        // Bank $00:F000 -> ROM offset 0x400000 + 0x7000 = 0x407000
        rom[0x407000] = 0x33;
        
        // Bank $01:8000 -> ROM offset 0x410000
        rom[0x410000] = 0x44;
        
        // Bank $02:A000 -> ROM offset 0x420000 + 0x2000 = 0x422000
        rom[0x422000] = 0x55;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // Verify address translation for bank $00 upper half
        assert_eq!(memory.read(0x008000), 0x11); // $00:8000 -> ROM $400000
        assert_eq!(memory.read(0x00C000), 0x22); // $00:C000 -> ROM $404000
        assert_eq!(memory.read(0x00F000), 0x33); // $00:F000 -> ROM $407000
        
        // Verify address translation for bank $01
        assert_eq!(memory.read(0x018000), 0x44); // $01:8000 -> ROM $410000
        
        // Verify address translation for bank $02
        assert_eq!(memory.read(0x02A000), 0x55); // $02:A000 -> ROM $422000
        
        // Verify mirroring in banks $80+
        assert_eq!(memory.read(0x808000), 0x11); // $80:8000 mirrors $00:8000
        assert_eq!(memory.read(0x818000), 0x44); // $81:8000 mirrors $01:8000
    }
    
    #[test]
    fn test_exhirom_word_access() {
        let rom = create_test_rom_exhirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test 16-bit write and read in WRAM
        memory.write_word(0x7E0000, 0xABCD);
        assert_eq!(memory.read_word(0x7E0000), 0xABCD);
        assert_eq!(memory.read(0x7E0000), 0xCD); // Little-endian
        assert_eq!(memory.read(0x7E0001), 0xAB);
        
        // Test 16-bit access in SRAM
        memory.write_word(0x006000, 0x1234);
        assert_eq!(memory.read_word(0x006000), 0x1234);
    }
}
