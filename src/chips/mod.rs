/// SNES Coprocessor/Special Chip Support
///
/// This module provides implementations of various SNES enhancement chips and coprocessors
/// including DSP-1, SA-1, and SuperFX.

pub mod dsp;
pub mod sa1;
pub mod superfx;

pub use dsp::Dsp1;
pub use sa1::Sa1;
pub use superfx::SuperFx;

/// Common interface for all SNES coprocessors
///
/// All enhancement chips must implement this trait to integrate with the
/// memory system and emulator timing loop.
pub trait CoProcessor: Send {
    /// Reset the coprocessor to its initial state
    fn reset(&mut self);

    /// Read a byte from the coprocessor's address space
    ///
    /// # Arguments
    /// * `addr` - 24-bit SNES address
    ///
    /// # Returns
    /// The byte value at the given address, or 0 for unmapped regions
    fn read(&mut self, addr: u32) -> u8;

    /// Write a byte to the coprocessor's address space
    ///
    /// # Arguments
    /// * `addr` - 24-bit SNES address
    /// * `val` - Byte value to write
    fn write(&mut self, addr: u32, val: u8);

    /// Execute the coprocessor for the given number of master cycles
    ///
    /// # Arguments
    /// * `cycles` - Number of master clock cycles to execute
    ///
    /// # Returns
    /// The actual number of cycles consumed (may differ for cycle-accurate chips)
    fn step(&mut self, cycles: u32) -> u32;

    /// Check if this coprocessor handles the given address
    ///
    /// # Arguments
    /// * `addr` - 24-bit SNES address
    ///
    /// # Returns
    /// true if this coprocessor should handle reads/writes to this address
    fn handles_address(&self, addr: u32) -> bool;
}

/// Types of SNES coprocessors that can be detected from cartridge headers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipType {
    /// DSP-1, DSP-2, DSP-3, or DSP-4 (Math/Coordinate coprocessor)
    Dsp1,
    /// SA-1 (Second 65816 CPU with extended features)
    Sa1,
    /// SuperFX/GSU (3D graphics coprocessor)
    SuperFx,
    /// CX4 (Wireframe 3D processor)
    Cx4,
    /// S-DD1 (Graphics decompression)
    Sdd1,
    /// SPC7110 (Data decompression/RTC)
    Spc7110,
    /// OBC1 (Memory controller for Metal Combat)
    Obc1,
    /// Unknown or unsupported chip
    Unknown(u8),
}

impl ChipType {
    /// Detect the chip type from the cartridge type byte (header offset 0xFFD6)
    ///
    /// # Arguments
    /// * `cartridge_type_byte` - The raw byte from the cartridge header
    ///
    /// # Returns
    /// The corresponding ChipType, or Unknown if not recognized
    pub fn from_cartridge_byte(cartridge_type_byte: u8) -> Option<Self> {
        match cartridge_type_byte {
            // DSP variants
            0x03 | 0x04 | 0x05 | 0x06 => Some(ChipType::Dsp1),
            
            // SuperFX variants
            0x13 | 0x14 | 0x15 | 0x1A => Some(ChipType::SuperFx),
            
            // SA-1 variants
            0x23 | 0x33 | 0x34 | 0x35 | 0x36 => Some(ChipType::Sa1),
            
            // S-DD1 variants
            0xE3 | 0xE4 | 0xE5 => Some(ChipType::Sdd1),
            
            // CX4
            0xF3 => Some(ChipType::Cx4),
            
            // SPC7110 variants
            0xF5 | 0xF6 | 0xF9 => Some(ChipType::Spc7110),
            
            // OBC1 (0x25 conflicts with SA-1, prioritize SA-1)
            // 0x25 => Some(ChipType::Obc1),
            
            _ => None,
        }
    }
}

/// Factory function to create a coprocessor instance based on chip type
///
/// # Arguments
/// * `chip_type` - The type of chip to instantiate
///
/// # Returns
/// A boxed trait object implementing CoProcessor, or None if unsupported
pub fn create_coprocessor(chip_type: ChipType) -> Option<Box<dyn CoProcessor>> {
    match chip_type {
        ChipType::Dsp1 => Some(Box::new(Dsp1::new())),
        ChipType::Sa1 => Some(Box::new(Sa1::new())),
        ChipType::SuperFx => Some(Box::new(SuperFx::new())),
        // Unimplemented chips return None
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_type_detection() {
        assert_eq!(ChipType::from_cartridge_byte(0x03), Some(ChipType::Dsp1));
        assert_eq!(ChipType::from_cartridge_byte(0x13), Some(ChipType::SuperFx));
        assert_eq!(ChipType::from_cartridge_byte(0x23), Some(ChipType::Sa1));
        assert_eq!(ChipType::from_cartridge_byte(0xF3), Some(ChipType::Cx4));
        assert_eq!(ChipType::from_cartridge_byte(0x00), None);
    }

    #[test]
    fn test_create_coprocessor() {
        // Should successfully create implemented chips
        assert!(create_coprocessor(ChipType::Dsp1).is_some());
        assert!(create_coprocessor(ChipType::Sa1).is_some());
        assert!(create_coprocessor(ChipType::SuperFx).is_some());
        
        // Unimplemented chips should return None
        assert!(create_coprocessor(ChipType::Cx4).is_none());
    }
}
