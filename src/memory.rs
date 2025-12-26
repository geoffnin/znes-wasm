use crate::apu::Apu;
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

    /// Audio subsystem (SPC700 + DSP)
    pub apu: Apu,
    
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
            apu: Apu::new(),
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
        // APU I/O ports ($2140-$2143)
        if (0x2140..=0x2143).contains(&(addr as u16)) {
            return self.apu.cpu_read_port(addr as u16);
        }

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
        // APU I/O ports ($2140-$2143)
        if (0x2140..=0x2143).contains(&(addr as u16)) {
            self.apu.cpu_write_port(addr as u16, value);
            return;
        }

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

    /// Get immutable APU reference
    pub fn apu(&self) -> &Apu {
        &self.apu
    }

    /// Get mutable APU reference
    pub fn apu_mut(&mut self) -> &mut Apu {
        &mut self.apu
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
    
    fn create_test_rom_hirom() -> Vec<u8> {
        let mut rom = vec![0; 0x10000]; // 64KB test ROM
        
        // Write header at offset $FFC0
        let header_offset = 0xFFC0;
        
        // ROM title (exactly 21 bytes)
        let title = b"HIROM TEST           "; // 21 bytes
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        
        // Mapping mode: HiROM ($21)
        rom[header_offset + 0x15] = 0x21;
        
        // Cartridge type
        rom[header_offset + 0x16] = 0x00;
        
        // ROM size (64KB = $09)
        rom[header_offset + 0x17] = 0x09;
        
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
    
    fn create_test_rom_exhirom() -> Vec<u8> {
        let mut rom = vec![0; 0x800000]; // 8MB test ROM for ExHiROM
        
        // Write header at offset $FFC0
        let header_offset = 0xFFC0;
        
        // ROM title (exactly 21 bytes)
        let title = b"EXHIROM TEST         "; // 21 bytes
        rom[header_offset..header_offset + 21].copy_from_slice(title);
        
        // Mapping mode: ExHiROM ($25)
        rom[header_offset + 0x15] = 0x25;
        
        // Cartridge type
        rom[header_offset + 0x16] = 0x00;
        
        // ROM size (8MB = $0C)
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
    
    // HiROM Tests
    
    #[test]
    fn test_memory_hirom_wram_access() {
        let rom = create_test_rom_hirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test WRAM write and read at $7E0000
        memory.write(0x7E0000, 0x42);
        assert_eq!(memory.read(0x7E0000), 0x42);
        
        // Test WRAM mirror at $000000-$001FFF
        memory.write(0x000100, 0xAB);
        assert_eq!(memory.read(0x000100), 0xAB);
        
        // Test WRAM mirror in bank $80
        memory.write(0x800200, 0xCD);
        assert_eq!(memory.read(0x800200), 0xCD);
        
        // Test WRAM at $7F0000
        memory.write(0x7F1234, 0xEF);
        assert_eq!(memory.read(0x7F1234), 0xEF);
    }
    
    #[test]
    fn test_memory_hirom_rom_access() {
        let mut rom = create_test_rom_hirom();
        // Write test data at various ROM offsets
        rom[0] = 0x12;
        rom[0x100] = 0x34;
        rom[0x8000] = 0x56;
        rom[0xC000] = 0x78;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // HiROM: Banks $00-$3F/$80-$BF have ROM in upper half ($8000-$FFFF)
        // Bank $00, offset $8000-$FFFF maps to ROM offset $0000-$7FFF
        assert_eq!(memory.read(0x008000), 0x12); // ROM offset 0
        assert_eq!(memory.read(0x008100), 0x34); // ROM offset 0x100
        
        // Test within bank $00
        // Address 0x00C000: page_in_bank = 6, rom_offset = 0 * 0x10000 + (6 - 4) * 0x2000 = 0x4000
        assert_eq!(memory.read(0x00C000), 0x00); // Offset (6-4)*0x2000 = 0x4000 (uninitialized)
        assert_eq!(memory.read(0x00E000), 0x00); // Offset (7-4)*0x2000 = 0x6000 (uninitialized)
        
        // Mirror in bank $80
        assert_eq!(memory.read(0x808000), 0x12);
        assert_eq!(memory.read(0x808100), 0x34);
    }
    
    #[test]
    fn test_memory_hirom_sram_access() {
        let rom = create_test_rom_hirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // HiROM SRAM is in banks $00-$3F at $6000-$7FFF
        // Write to bank $00, address $6000
        memory.write(0x006000, 0xAA);
        assert_eq!(memory.read(0x006000), 0xAA);
        
        // Test SRAM mirror in bank $20
        memory.write(0x206000, 0xBB);
        // Due to mirroring modulo, this should wrap
        assert_eq!(memory.read(0x206000), 0xBB);
        
        // Test SRAM save/load
        let sram_data = memory.sram();
        assert_eq!(sram_data.len(), 8192); // 8KB as specified in header
    }
    
    // ExHiROM Tests
    
    #[test]
    fn test_memory_exhirom_wram_access() {
        let rom = create_test_rom_exhirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // Test WRAM write and read at $7E0000
        memory.write(0x7E0000, 0x42);
        assert_eq!(memory.read(0x7E0000), 0x42);
        
        // Test WRAM mirror at $000000-$001FFF
        memory.write(0x000100, 0xAB);
        assert_eq!(memory.read(0x000100), 0xAB);
        
        // Test WRAM mirror in bank $80
        memory.write(0x800200, 0xCD);
        assert_eq!(memory.read(0x800200), 0xCD);
        
        // Test WRAM at $7F0000
        memory.write(0x7F5678, 0xEF);
        assert_eq!(memory.read(0x7F5678), 0xEF);
    }
    
    #[test]
    fn test_memory_exhirom_rom_access() {
        let mut rom = create_test_rom_exhirom();
        // Write test data at various ROM offsets for ExHiROM
        // ExHiROM banks $00-$3F upper half map to ROM offset $400000+
        rom[0x400000] = 0x12;
        rom[0x400100] = 0x34;
        rom[0x500000] = 0x78; // For bank $50 access
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let memory = Memory::new(&cartridge);
        
        // ExHiROM: Banks $00-$3F/$80-$BF, upper half ($8000-$FFFF)
        // Maps to ROM with offset $400000+
        // Bank $00, offset $8000: effective_bank=0
        // rom_offset = (0 + 0x40) * 0x10000 + (4 - 4) * 0x2000 = 0x400000
        assert_eq!(memory.read(0x008000), 0x12); // ROM offset 0x400000
        assert_eq!(memory.read(0x008100), 0x34); // ROM offset 0x400100
        
        // Banks $40-$7D map directly
        // Bank $40, offset $0: rom_offset = 0x40 * 0x10000 + 0 = 0x400000
        assert_eq!(memory.read(0x400000), 0x12); // ROM offset 0x400000
        
        // Test bank $50: rom_offset = 0x50 * 0x10000 = 0x500000
        assert_eq!(memory.read(0x500000), 0x78); // ROM offset 0x500000
        
        // Test mirror in bank $80-$BF
        // Bank $80, offset $8000: effective_bank = 0x80 & 0x3F = 0
        // rom_offset = (0 + 0x40) * 0x10000 + 0 = 0x400000
        assert_eq!(memory.read(0x808000), 0x12); // Mirror of bank $00
    }
    
    #[test]
    fn test_memory_exhirom_sram_access() {
        let rom = create_test_rom_exhirom();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        let mut memory = Memory::new(&cartridge);
        
        // ExHiROM SRAM is in banks $00-$3F at $6000-$7FFF (similar to HiROM)
        memory.write(0x006000, 0xAA);
        assert_eq!(memory.read(0x006000), 0xAA);
        
        // Test SRAM mirror
        memory.write(0x106000, 0xBB);
        assert_eq!(memory.read(0x106000), 0xBB);
        
        // Test SRAM save/load
        let sram_data = memory.sram();
        assert_eq!(sram_data.len(), 8192); // 8KB as specified in header
    }
}
