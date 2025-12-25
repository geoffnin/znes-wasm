/// SNES Cartridge ROM loading and header parsing
/// 
/// Supports .sfc and .smc formats (with optional 512-byte headers)
/// Parses ROM headers to detect mapping mode, region, and other metadata

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MappingMode {
    LoRom,    // Low ROM mapping
    HiRom,    // High ROM mapping  
    ExHiRom,  // Extended High ROM mapping
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    Japan,
    NorthAmerica,
    Europe,
    Sweden,
    Finland,
    Denmark,
    France,
    Netherlands,
    Spain,
    Germany,
    Italy,
    China,
    Indonesia,
    Korea,
    Common,
    Canada,
    Brazil,
    Australia,
    Other(u8),
}

impl From<u8> for Region {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Region::Japan,
            0x01 => Region::NorthAmerica,
            0x02 => Region::Europe,
            0x03 => Region::Sweden,
            0x04 => Region::Finland,
            0x05 => Region::Denmark,
            0x06 => Region::France,
            0x07 => Region::Netherlands,
            0x08 => Region::Spain,
            0x09 => Region::Germany,
            0x0A => Region::Italy,
            0x0B => Region::China,
            0x0C => Region::Indonesia,
            0x0D => Region::Korea,
            0x0E => Region::Common,
            0x0F => Region::Canada,
            0x10 => Region::Brazil,
            0x11 => Region::Australia,
            other => Region::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CartridgeType {
    RomOnly,
    RomRam,
    RomRamBattery,
    RomCoprocessor(u8),
}

pub struct Cartridge {
    /// ROM data (without header)
    rom_data: Vec<u8>,
    
    /// Mapping mode detected from header
    mapping_mode: MappingMode,
    
    /// ROM title (21 characters)
    title: String,
    
    /// Region code
    region: Region,
    
    /// Cartridge type
    cartridge_type: CartridgeType,
    
    /// ROM size in bytes
    rom_size: usize,
    
    /// SRAM size in bytes
    sram_size: usize,
    
    /// Header was valid
    header_valid: bool,
}

impl Cartridge {
    /// Load a ROM from raw data
    /// Automatically detects and strips .smc headers
    pub fn from_rom(data: Vec<u8>) -> Result<Self, String> {
        // Check if ROM has a 512-byte header (.smc format)
        let has_header = data.len() % 1024 == 512;
        
        let rom_data = if has_header {
            // Skip the 512-byte header
            data[512..].to_vec()
        } else {
            data
        };
        
        // Try to detect mapping mode and parse header
        let (_mapping_mode, header_offset) = Self::detect_mapping_mode(&rom_data)?;
        
        // Parse the header
        let header = Self::parse_header(&rom_data, header_offset)?;
        
        Ok(Cartridge {
            rom_data,
            mapping_mode: header.mapping_mode,
            title: header.title,
            region: header.region,
            cartridge_type: header.cartridge_type,
            rom_size: header.rom_size,
            sram_size: header.sram_size,
            header_valid: true,
        })
    }
    
    /// Detect the mapping mode by checking header locations
    fn detect_mapping_mode(rom_data: &[u8]) -> Result<(MappingMode, usize), String> {
        // LoROM header is at $7FC0-$7FFF (offset $7FC0)
        // HiROM header is at $FFC0-$FFFF (offset $FFC0)
        
        let lorom_offset = 0x7FC0;
        let hirom_offset = 0xFFC0;
        
        // Score each potential header location
        let lorom_score = if rom_data.len() > lorom_offset + 0x30 {
            Self::score_header(rom_data, lorom_offset)
        } else {
            0
        };
        
        let hirom_score = if rom_data.len() > hirom_offset + 0x30 {
            Self::score_header(rom_data, hirom_offset)
        } else {
            0
        };
        
        // Choose the header with the highest score
        if lorom_score > hirom_score && lorom_score > 0 {
            let map_mode_byte = rom_data[lorom_offset + 0x15];
            let mapping = Self::parse_mapping_mode(map_mode_byte);
            Ok((mapping, lorom_offset))
        } else if hirom_score > 0 {
            let map_mode_byte = rom_data[hirom_offset + 0x15];
            let mapping = Self::parse_mapping_mode(map_mode_byte);
            Ok((mapping, hirom_offset))
        } else if rom_data.len() > lorom_offset + 0x30 {
            // Default to LoROM if we have a valid LoROM header location
            Ok((MappingMode::LoRom, lorom_offset))
        } else {
            Err("ROM too small to contain valid header".to_string())
        }
    }
    
    /// Score a potential header location based on validity checks
    fn score_header(rom_data: &[u8], offset: usize) -> u32 {
        let mut score = 0;
        
        // Check mapping mode byte (should be in valid range)
        let map_mode = rom_data[offset + 0x15];
        if map_mode >= 0x20 && map_mode <= 0x35 {
            score += 2;
        }
        
        // Check cartridge type (should be reasonable)
        let cart_type = rom_data[offset + 0x16];
        if cart_type <= 0x0F {
            score += 1;
        }
        
        // Check ROM size (should be reasonable: $08-$0D typically)
        let rom_size = rom_data[offset + 0x17];
        if rom_size >= 0x08 && rom_size <= 0x0D {
            score += 2;
        }
        
        // Check SRAM size (should be $00-$08 typically)
        let sram_size = rom_data[offset + 0x18];
        if sram_size <= 0x08 {
            score += 1;
        }
        
        // Check region code
        let region = rom_data[offset + 0x19];
        if region <= 0x14 {
            score += 1;
        }
        
        // Check checksum complement
        let checksum_lo = rom_data[offset + 0x1C] as u16;
        let checksum_hi = rom_data[offset + 0x1D] as u16;
        let checksum_comp_lo = rom_data[offset + 0x1E] as u16;
        let checksum_comp_hi = rom_data[offset + 0x1F] as u16;
        
        let checksum = checksum_lo | (checksum_hi << 8);
        let checksum_comp = checksum_comp_lo | (checksum_comp_hi << 8);
        
        if checksum ^ checksum_comp == 0xFFFF {
            score += 4; // Checksum complement is very good indicator
        }
        
        score
    }
    
    /// Parse the mapping mode byte
    fn parse_mapping_mode(mode_byte: u8) -> MappingMode {
        let _speed = mode_byte & 0x10; // Fast/Slow ROM (not used in detection)
        let mode = mode_byte & 0x0F;
        
        match mode {
            0x00 => MappingMode::LoRom,
            0x01 => MappingMode::HiRom,
            0x05 => MappingMode::ExHiRom,
            _ => MappingMode::LoRom, // Default
        }
    }
    
    /// Parse the ROM header at the given offset
    fn parse_header(rom_data: &[u8], offset: usize) -> Result<HeaderInfo, String> {
        if rom_data.len() < offset + 0x30 {
            return Err("ROM too small for header".to_string());
        }
        
        // Extract title (21 bytes, ASCII)
        let title_bytes = &rom_data[offset..offset + 21];
        let title = String::from_utf8_lossy(title_bytes)
            .trim_end()
            .to_string();
        
        // Mapping mode
        let map_mode_byte = rom_data[offset + 0x15];
        let mapping_mode = Self::parse_mapping_mode(map_mode_byte);
        
        // Cartridge type
        let cart_type_byte = rom_data[offset + 0x16];
        let cartridge_type = match cart_type_byte {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::RomRam,
            0x02 => CartridgeType::RomRamBattery,
            0x03..=0x06 => CartridgeType::RomCoprocessor(cart_type_byte),
            _ => CartridgeType::RomOnly,
        };
        
        // ROM size (2^n KB)
        let rom_size_byte = rom_data[offset + 0x17];
        let rom_size = if rom_size_byte < 16 {
            1024 << rom_size_byte
        } else {
            rom_data.len() // Use actual size if invalid
        };
        
        // SRAM size (2^n KB)
        let sram_size_byte = rom_data[offset + 0x18];
        let sram_size = if sram_size_byte > 0 && sram_size_byte < 16 {
            1024 << sram_size_byte
        } else {
            0
        };
        
        // Region
        let region_byte = rom_data[offset + 0x19];
        let region = Region::from(region_byte);
        
        Ok(HeaderInfo {
            mapping_mode,
            title,
            region,
            cartridge_type,
            rom_size,
            sram_size,
        })
    }
    
    /// Get the ROM data
    pub fn rom_data(&self) -> &[u8] {
        &self.rom_data
    }
    
    /// Get the mapping mode
    pub fn mapping_mode(&self) -> MappingMode {
        self.mapping_mode
    }
    
    /// Get the ROM title
    pub fn title(&self) -> &str {
        &self.title
    }
    
    /// Get the region
    pub fn region(&self) -> Region {
        self.region
    }
    
    /// Get the cartridge type
    pub fn cartridge_type(&self) -> CartridgeType {
        self.cartridge_type
    }
    
    /// Get the ROM size in bytes
    pub fn rom_size(&self) -> usize {
        self.rom_size
    }
    
    /// Get the SRAM size in bytes
    pub fn sram_size(&self) -> usize {
        self.sram_size
    }
    
    /// Check if header was valid
    pub fn is_header_valid(&self) -> bool {
        self.header_valid
    }
}

struct HeaderInfo {
    mapping_mode: MappingMode,
    title: String,
    region: Region,
    cartridge_type: CartridgeType,
    rom_size: usize,
    sram_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_lorom_header() -> Vec<u8> {
        let mut rom = vec![0; 0x8000]; // 32KB
        
        // Header at $7FC0
        let offset = 0x7FC0;
        
        // Title: "TEST ROM" (padded with spaces to exactly 21 bytes)
        let title = b"TEST ROM             "; // 21 bytes
        rom[offset..offset + 21].copy_from_slice(title);
        
        // Mapping mode: LoROM, Fast ($20)
        rom[offset + 0x15] = 0x20;
        
        // Cartridge type: ROM only
        rom[offset + 0x16] = 0x00;
        
        // ROM size: 32KB = 2^15 = $08
        rom[offset + 0x17] = 0x08;
        
        // SRAM size: 8KB = 2^13 = $03
        rom[offset + 0x18] = 0x03;
        
        // Region: North America
        rom[offset + 0x19] = 0x01;
        
        // Checksum complement
        rom[offset + 0x1C] = 0x00;
        rom[offset + 0x1D] = 0x00;
        rom[offset + 0x1E] = 0xFF;
        rom[offset + 0x1F] = 0xFF;
        
        rom
    }
    
    fn create_hirom_header() -> Vec<u8> {
        let mut rom = vec![0; 0x10000]; // 64KB
        
        // Header at $FFC0
        let offset = 0xFFC0;
        
        // Title (exactly 21 bytes)
        let title = b"HIROM TEST           "; // 21 bytes
        rom[offset..offset + 21].copy_from_slice(title);
        
        // Mapping mode: HiROM, Fast ($21)
        rom[offset + 0x15] = 0x21;
        
        // Cartridge type: ROM+RAM
        rom[offset + 0x16] = 0x01;
        
        // ROM size: 64KB = 2^16 = $09
        rom[offset + 0x17] = 0x09;
        
        // SRAM size: None
        rom[offset + 0x18] = 0x00;
        
        // Region: Japan
        rom[offset + 0x19] = 0x00;
        
        // Checksum complement
        rom[offset + 0x1C] = 0x12;
        rom[offset + 0x1D] = 0x34;
        rom[offset + 0x1E] = 0xED;
        rom[offset + 0x1F] = 0xCB;
        
        rom
    }
    
    #[test]
    fn test_lorom_detection() {
        let rom = create_lorom_header();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        
        assert_eq!(cartridge.mapping_mode(), MappingMode::LoRom);
        assert_eq!(cartridge.title(), "TEST ROM");
        assert_eq!(cartridge.region(), Region::NorthAmerica);
        assert_eq!(cartridge.sram_size(), 8192);
    }
    
    #[test]
    fn test_hirom_detection() {
        let rom = create_hirom_header();
        let cartridge = Cartridge::from_rom(rom).unwrap();
        
        assert_eq!(cartridge.mapping_mode(), MappingMode::HiRom);
        assert_eq!(cartridge.title(), "HIROM TEST");
        assert_eq!(cartridge.region(), Region::Japan);
    }
    
    #[test]
    fn test_smc_header_removal() {
        let mut rom_with_header = vec![0; 512]; // 512-byte .smc header
        let mut rom = create_lorom_header();
        rom_with_header.append(&mut rom);
        
        let cartridge = Cartridge::from_rom(rom_with_header).unwrap();
        
        assert_eq!(cartridge.mapping_mode(), MappingMode::LoRom);
        assert_eq!(cartridge.title(), "TEST ROM");
        assert_eq!(cartridge.rom_data().len(), 0x8000);
    }
    
    #[test]
    fn test_cartridge_type_detection() {
        let mut rom = create_lorom_header();
        
        // Set cartridge type to ROM+RAM+Battery
        rom[0x7FC0 + 0x16] = 0x02;
        
        let cartridge = Cartridge::from_rom(rom).unwrap();
        assert_eq!(cartridge.cartridge_type(), CartridgeType::RomRamBattery);
    }
    
    #[test]
    fn test_region_detection() {
        let mut rom = create_lorom_header();
        
        // Test various regions
        rom[0x7FC0 + 0x19] = 0x02; // Europe
        let cartridge = Cartridge::from_rom(rom.clone()).unwrap();
        assert_eq!(cartridge.region(), Region::Europe);
        
        rom[0x7FC0 + 0x19] = 0x00; // Japan
        let cartridge = Cartridge::from_rom(rom.clone()).unwrap();
        assert_eq!(cartridge.region(), Region::Japan);
    }
}
