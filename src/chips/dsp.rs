/// DSP-1 Math Coprocessor Implementation
///
/// The DSP-1 (NEC µPD77C25) is a 16-bit fixed-point math coprocessor used in games like
/// Super Mario Kart, Pilotwings, and F-Zero. It provides fast multiplication, division,
/// inverse, square root, and coordinate transformation operations (Mode 7 support).
///
/// Memory Map (LoROM):
/// - 0x6000-0x6FFF: Data Register (read/write)
/// - 0x7000-0x7FFF: Status Register (read only)

use super::CoProcessor;

/// DSP-1 Commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dsp1Command {
    Multiply = 0x00,        // 16x16 signed multiply
    Inverse = 0x04,         // 1/x inverse
    SquareRoot = 0x0C,      // Square root
    Attitude = 0x06,        // 3D attitude/rotation (Mode 7)
    Objective = 0x08,       // Target/objective calculation
    SubjectiveA = 0x0A,     // Subjective view angle A
    SubjectiveB = 0x0E,     // Subjective view angle B
    Radius = 0x02,          // Distance calculation
    Range = 0x0B,           // Range/vector calculation
    Distance = 0x01,        // 2D distance
    Rotate = 0x05,          // 2D rotation
    Project = 0x07,         // 3D projection
    ParameterA = 0x0F,      // Parameter load A
    ParameterB = 0x09,      // Parameter load B
    ParameterC = 0x0D,      // Parameter load C
    MemTest = 0x03,         // Memory test
}

impl Dsp1Command {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::Multiply),
            0x04 => Some(Self::Inverse),
            0x0C => Some(Self::SquareRoot),
            0x06 => Some(Self::Attitude),
            0x08 => Some(Self::Objective),
            0x0A => Some(Self::SubjectiveA),
            0x0E => Some(Self::SubjectiveB),
            0x02 => Some(Self::Radius),
            0x0B => Some(Self::Range),
            0x01 => Some(Self::Distance),
            0x05 => Some(Self::Rotate),
            0x07 => Some(Self::Project),
            0x0F => Some(Self::ParameterA),
            0x09 => Some(Self::ParameterB),
            0x0D => Some(Self::ParameterC),
            0x03 => Some(Self::MemTest),
            _ => None,
        }
    }

    fn input_size(&self) -> usize {
        match self {
            Self::Multiply => 4,        // 2 x 16-bit operands
            Self::Inverse => 2,         // 1 x 16-bit operand
            Self::SquareRoot => 2,      // 1 x 16-bit operand
            Self::Attitude => 8,        // 4 x 16-bit parameters
            Self::Objective => 6,       // 3 x 16-bit parameters
            Self::SubjectiveA => 6,     // 3 x 16-bit parameters
            Self::SubjectiveB => 6,     // 3 x 16-bit parameters
            Self::Radius => 4,          // 2 x 16-bit coordinates
            Self::Range => 4,           // 2 x 16-bit coordinates
            Self::Distance => 6,        // 3 x 16-bit coordinates
            Self::Rotate => 4,          // 2 x 16-bit angle + value
            Self::Project => 6,         // 3 x 16-bit coordinates
            Self::ParameterA => 2,      // 1 x 16-bit parameter
            Self::ParameterB => 2,      // 1 x 16-bit parameter
            Self::ParameterC => 2,      // 1 x 16-bit parameter
            Self::MemTest => 2,         // 1 x 16-bit test value
        }
    }

    fn output_size(&self) -> usize {
        match self {
            Self::Multiply => 4,        // 32-bit result
            Self::Inverse => 2,         // 16-bit result
            Self::SquareRoot => 2,      // 16-bit result
            Self::Attitude => 8,        // 4 x 16-bit rotation matrix
            Self::Objective => 6,       // 3 x 16-bit coordinates
            Self::SubjectiveA => 6,     // 3 x 16-bit angles
            Self::SubjectiveB => 6,     // 3 x 16-bit angles
            Self::Radius => 2,          // 16-bit distance
            Self::Range => 2,           // 16-bit distance
            Self::Distance => 2,        // 16-bit distance
            Self::Rotate => 4,          // 2 x 16-bit rotated coordinates
            Self::Project => 4,         // 2 x 16-bit screen coordinates
            Self::ParameterA => 0,      // No output
            Self::ParameterB => 0,      // No output
            Self::ParameterC => 0,      // No output
            Self::MemTest => 2,         // 16-bit test result
        }
    }
}

/// DSP-1 Coprocessor State
pub struct Dsp1 {
    /// Current command being executed
    current_command: Option<Dsp1Command>,
    
    /// Input parameters buffer
    input_buffer: Vec<u8>,
    
    /// Output results buffer
    output_buffer: Vec<u8>,
    
    /// Current read/write position in buffers
    buffer_position: usize,
    
    /// Busy flag (false = ready, true = busy/processing)
    busy: bool,
    
    /// Coordinate transformation matrix (for Mode 7 operations)
    attitude_matrix: [[i16; 3]; 3],
    
    /// Stored parameters for complex operations
    parameters: [i16; 16],
}

impl Dsp1 {
    pub fn new() -> Self {
        Self {
            current_command: None,
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            buffer_position: 0,
            busy: false,
            attitude_matrix: [[0; 3]; 3],
            parameters: [0; 16],
        }
    }

    /// Execute the current command with the buffered input
    fn execute_command(&mut self) {
        if let Some(cmd) = self.current_command {
            self.output_buffer.clear();
            
            match cmd {
                Dsp1Command::Multiply => self.cmd_multiply(),
                Dsp1Command::Inverse => self.cmd_inverse(),
                Dsp1Command::SquareRoot => self.cmd_square_root(),
                Dsp1Command::Attitude => self.cmd_attitude(),
                Dsp1Command::Objective => self.cmd_objective(),
                Dsp1Command::SubjectiveA => self.cmd_subjective_a(),
                Dsp1Command::SubjectiveB => self.cmd_subjective_b(),
                Dsp1Command::Radius => self.cmd_radius(),
                Dsp1Command::Range => self.cmd_range(),
                Dsp1Command::Distance => self.cmd_distance(),
                Dsp1Command::Rotate => self.cmd_rotate(),
                Dsp1Command::Project => self.cmd_project(),
                Dsp1Command::ParameterA => self.cmd_parameter_a(),
                Dsp1Command::ParameterB => self.cmd_parameter_b(),
                Dsp1Command::ParameterC => self.cmd_parameter_c(),
                Dsp1Command::MemTest => self.cmd_mem_test(),
            }
            
            self.buffer_position = 0;
            self.busy = false;
        }
    }

    /// Read a 16-bit signed value from the input buffer
    fn read_input_i16(&self, offset: usize) -> i16 {
        let lo = self.input_buffer[offset] as u16;
        let hi = self.input_buffer[offset + 1] as u16;
        ((hi << 8) | lo) as i16
    }

    /// Write a 16-bit signed value to the output buffer
    fn write_output_i16(&mut self, value: i16) {
        self.output_buffer.push((value & 0xFF) as u8);
        self.output_buffer.push(((value >> 8) & 0xFF) as u8);
    }

    /// Write a 32-bit signed value to the output buffer
    fn write_output_i32(&mut self, value: i32) {
        self.output_buffer.push((value & 0xFF) as u8);
        self.output_buffer.push(((value >> 8) & 0xFF) as u8);
        self.output_buffer.push(((value >> 16) & 0xFF) as u8);
        self.output_buffer.push(((value >> 24) & 0xFF) as u8);
    }

    // ===== Math Operations =====

    /// Command 0x00: Multiply two 16-bit signed integers
    fn cmd_multiply(&mut self) {
        let a = self.read_input_i16(0) as i32;
        let b = self.read_input_i16(2) as i32;
        let result = a * b;
        self.write_output_i32(result);
    }

    /// Command 0x04: Calculate inverse (1/x) using fixed-point
    fn cmd_inverse(&mut self) {
        let x = self.read_input_i16(0) as i32;
        if x == 0 {
            self.write_output_i16(0x7FFF); // Max positive value for division by zero
        } else {
            // Fixed-point inverse: (1 << 16) / x
            let result = ((1i32 << 16) / x) as i16;
            self.write_output_i16(result);
        }
    }

    /// Command 0x0C: Calculate square root
    fn cmd_square_root(&mut self) {
        let x = self.read_input_i16(0) as u16;
        let result = (x as f64).sqrt() as u16 as i16;
        self.write_output_i16(result);
    }

    // ===== Coordinate Transformation Operations (Mode 7) =====

    /// Command 0x06: Attitude - Calculate 3D rotation matrix
    /// Used by Super Mario Kart for Mode 7 transformations
    fn cmd_attitude(&mut self) {
        let pitch = self.read_input_i16(0);
        let yaw = self.read_input_i16(2);
        let roll = self.read_input_i16(4);
        let range = self.read_input_i16(6);

        // Convert to radians (simple approximation)
        let pitch_rad = (pitch as f64) * std::f64::consts::PI / 32768.0;
        let yaw_rad = (yaw as f64) * std::f64::consts::PI / 32768.0;
        let roll_rad = (roll as f64) * std::f64::consts::PI / 32768.0;

        let cos_p = pitch_rad.cos();
        let sin_p = pitch_rad.sin();
        let cos_y = yaw_rad.cos();
        let sin_y = yaw_rad.sin();
        let cos_r = roll_rad.cos();
        let sin_r = roll_rad.sin();

        // Calculate rotation matrix elements (simplified)
        self.attitude_matrix[0][0] = (cos_y * cos_r * 256.0) as i16;
        self.attitude_matrix[0][1] = ((-cos_y * sin_r) * 256.0) as i16;
        self.attitude_matrix[0][2] = (sin_y * 256.0) as i16;
        
        self.attitude_matrix[1][0] = ((sin_p * sin_y * cos_r + cos_p * sin_r) * 256.0) as i16;
        self.attitude_matrix[1][1] = ((-sin_p * sin_y * sin_r + cos_p * cos_r) * 256.0) as i16;
        self.attitude_matrix[1][2] = ((-sin_p * cos_y) * 256.0) as i16;
        
        self.attitude_matrix[2][0] = ((cos_p * sin_y * cos_r - sin_p * sin_r) * 256.0) as i16;
        self.attitude_matrix[2][1] = ((-cos_p * sin_y * sin_r - sin_p * cos_r) * 256.0) as i16;
        self.attitude_matrix[2][2] = ((cos_p * cos_y) * 256.0) as i16;

        // Output the matrix elements
        for row in 0..3 {
            for col in 0..2 {
                self.write_output_i16(self.attitude_matrix[row][col]);
            }
        }
        self.write_output_i16(range);
        self.write_output_i16(0); // Padding
    }

    /// Command 0x08: Objective - Transform world coordinates to screen
    fn cmd_objective(&mut self) {
        let x = self.read_input_i16(0);
        let y = self.read_input_i16(2);
        let z = self.read_input_i16(4);

        // Apply rotation matrix
        let rx = ((self.attitude_matrix[0][0] as i32 * x as i32 +
                   self.attitude_matrix[0][1] as i32 * y as i32 +
                   self.attitude_matrix[0][2] as i32 * z as i32) >> 8) as i16;
        
        let ry = ((self.attitude_matrix[1][0] as i32 * x as i32 +
                   self.attitude_matrix[1][1] as i32 * y as i32 +
                   self.attitude_matrix[1][2] as i32 * z as i32) >> 8) as i16;
        
        let rz = ((self.attitude_matrix[2][0] as i32 * x as i32 +
                   self.attitude_matrix[2][1] as i32 * y as i32 +
                   self.attitude_matrix[2][2] as i32 * z as i32) >> 8) as i16;

        self.write_output_i16(rx);
        self.write_output_i16(ry);
        self.write_output_i16(rz);
    }

    /// Command 0x0A: Subjective A - Calculate viewing angle (variant A)
    fn cmd_subjective_a(&mut self) {
        let x = self.read_input_i16(0);
        let y = self.read_input_i16(2);
        let z = self.read_input_i16(4);

        // Simplified subjective calculation
        let dist = ((x as i32 * x as i32 + y as i32 * y as i32 + z as i32 * z as i32) as f64).sqrt() as i16;
        let angle_h = ((y as f64).atan2(x as f64) * 32768.0 / std::f64::consts::PI) as i16;
        let angle_v = ((z as f64).atan2(dist as f64) * 32768.0 / std::f64::consts::PI) as i16;

        self.write_output_i16(dist);
        self.write_output_i16(angle_h);
        self.write_output_i16(angle_v);
    }

    /// Command 0x0E: Subjective B - Calculate viewing angle (variant B)
    fn cmd_subjective_b(&mut self) {
        // Similar to Subjective A but with different calculation
        self.cmd_subjective_a();
    }

    /// Command 0x02: Radius - Calculate 2D distance
    fn cmd_radius(&mut self) {
        let x = self.read_input_i16(0) as i32;
        let y = self.read_input_i16(2) as i32;
        let dist = ((x * x + y * y) as f64).sqrt() as i16;
        self.write_output_i16(dist);
    }

    /// Command 0x0B: Range - Calculate vector length
    fn cmd_range(&mut self) {
        self.cmd_radius(); // Same as radius for 2D
    }

    /// Command 0x01: Distance - Calculate 3D distance
    fn cmd_distance(&mut self) {
        let x = self.read_input_i16(0) as i32;
        let y = self.read_input_i16(2) as i32;
        let z = self.read_input_i16(4) as i32;
        let dist = ((x * x + y * y + z * z) as f64).sqrt() as i16;
        self.write_output_i16(dist);
    }

    /// Command 0x05: Rotate - 2D rotation
    fn cmd_rotate(&mut self) {
        let angle = self.read_input_i16(0);
        let value = self.read_input_i16(2);
        
        let angle_rad = (angle as f64) * std::f64::consts::PI / 32768.0;
        let cos_a = (angle_rad.cos() * 256.0) as i16;
        let sin_a = (angle_rad.sin() * 256.0) as i16;
        
        let x = ((cos_a as i32 * value as i32) >> 8) as i16;
        let y = ((sin_a as i32 * value as i32) >> 8) as i16;
        
        self.write_output_i16(x);
        self.write_output_i16(y);
    }

    /// Command 0x07: Project - 3D to 2D projection
    fn cmd_project(&mut self) {
        let x = self.read_input_i16(0) as i32;
        let y = self.read_input_i16(2) as i32;
        let z = self.read_input_i16(4) as i32;

        // Simple perspective projection
        let focal_length = 256;
        if z != 0 {
            let screen_x = ((x * focal_length) / z) as i16;
            let screen_y = ((y * focal_length) / z) as i16;
            self.write_output_i16(screen_x);
            self.write_output_i16(screen_y);
        } else {
            self.write_output_i16(0);
            self.write_output_i16(0);
        }
    }

    // ===== Parameter Commands =====

    /// Command 0x0F: Parameter A - Store parameter
    fn cmd_parameter_a(&mut self) {
        let value = self.read_input_i16(0);
        self.parameters[0] = value;
    }

    /// Command 0x09: Parameter B - Store parameter
    fn cmd_parameter_b(&mut self) {
        let value = self.read_input_i16(0);
        self.parameters[1] = value;
    }

    /// Command 0x0D: Parameter C - Store parameter
    fn cmd_parameter_c(&mut self) {
        let value = self.read_input_i16(0);
        self.parameters[2] = value;
    }

    /// Command 0x03: Memory Test - Echo back test value
    fn cmd_mem_test(&mut self) {
        let value = self.read_input_i16(0);
        self.write_output_i16(value);
    }
}

impl CoProcessor for Dsp1 {
    fn reset(&mut self) {
        self.current_command = None;
        self.input_buffer.clear();
        self.output_buffer.clear();
        self.buffer_position = 0;
        self.busy = false;
        self.attitude_matrix = [[0; 3]; 3];
        self.parameters = [0; 16];
    }

    fn read(&mut self, addr: u32) -> u8 {
        let addr = addr & 0xFFFF;
        
        match addr {
            // 0x6000-0x6FFF: Data Register
            0x6000..=0x6FFF => {
                if self.buffer_position < self.output_buffer.len() {
                    let value = self.output_buffer[self.buffer_position];
                    self.buffer_position += 1;
                    value
                } else {
                    0x00
                }
            }
            
            // 0x7000-0x7FFF: Status Register
            0x7000..=0x7FFF => {
                // Bit 7: Busy flag (0 = ready, 1 = busy)
                // Bit 6-0: Reserved
                if self.busy { 0x80 } else { 0x00 }
            }
            
            _ => 0x00,
        }
    }

    fn write(&mut self, addr: u32, val: u8) {
        let addr = addr & 0xFFFF;
        
        match addr {
            // 0x6000-0x6FFF: Data Register
            0x6000..=0x6FFF => {
                if self.current_command.is_none() {
                    // First byte is the command
                    if let Some(cmd) = Dsp1Command::from_byte(val) {
                        self.current_command = Some(cmd);
                        self.input_buffer.clear();
                        self.output_buffer.clear();
                        self.buffer_position = 0;
                        self.busy = true;
                    }
                } else if let Some(cmd) = self.current_command {
                    // Subsequent bytes are input parameters
                    self.input_buffer.push(val);
                    
                    // Execute when we have all input parameters
                    if self.input_buffer.len() >= cmd.input_size() {
                        self.execute_command();
                    }
                }
            }
            
            // 0x7000-0x7FFF: Status Register (read-only, writes ignored)
            0x7000..=0x7FFF => {}
            
            _ => {}
        }
    }

    fn step(&mut self, _cycles: u32) -> u32 {
        // DSP-1 operates instantly (no cycle-accurate timing needed)
        0
    }

    fn handles_address(&self, addr: u32) -> bool {
        let addr = addr & 0xFFFF;
        matches!(addr, 0x6000..=0x7FFF)
    }
}

impl Default for Dsp1 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiply() {
        let mut dsp = Dsp1::new();
        
        // Write multiply command
        dsp.write(0x6000, 0x00);
        
        // Write operands: 100 * 200
        dsp.write(0x6000, 100);
        dsp.write(0x6000, 0);
        dsp.write(0x6000, 200);
        dsp.write(0x6000, 0);
        
        // Read result (should be 20000 = 0x4E20)
        let lo = dsp.read(0x6000) as u32;
        let ml = dsp.read(0x6000) as u32;
        let mh = dsp.read(0x6000) as u32;
        let hi = dsp.read(0x6000) as u32;
        let result = lo | (ml << 8) | (mh << 16) | (hi << 24);
        
        assert_eq!(result, 20000);
    }

    #[test]
    fn test_square_root() {
        let mut dsp = Dsp1::new();
        
        // Write square root command
        dsp.write(0x6000, 0x0C);
        
        // Write operand: 144
        dsp.write(0x6000, 144);
        dsp.write(0x6000, 0);
        
        // Read result (should be 12)
        let lo = dsp.read(0x6000);
        let hi = dsp.read(0x6000);
        let result = (hi as u16) << 8 | (lo as u16);
        
        assert_eq!(result, 12);
    }

    #[test]
    fn test_radius() {
        let mut dsp = Dsp1::new();
        
        // Write radius command
        dsp.write(0x6000, 0x02);
        
        // Write coordinates: (3, 4) -> distance should be 5
        dsp.write(0x6000, 3);
        dsp.write(0x6000, 0);
        dsp.write(0x6000, 4);
        dsp.write(0x6000, 0);
        
        // Read result
        let lo = dsp.read(0x6000);
        let hi = dsp.read(0x6000);
        let result = (hi as u16) << 8 | (lo as u16);
        
        assert_eq!(result, 5);
    }

    #[test]
    fn test_status_register() {
        let mut dsp = Dsp1::new();
        
        // Initially not busy
        assert_eq!(dsp.read(0x7000), 0x00);
        
        // Start multiply command
        dsp.write(0x6000, 0x00);
        assert_eq!(dsp.read(0x7000), 0x80); // Busy
        
        // Complete the command
        dsp.write(0x6000, 1);
        dsp.write(0x6000, 0);
        dsp.write(0x6000, 1);
        dsp.write(0x6000, 0);
        
        // Should be not busy after completion
        assert_eq!(dsp.read(0x7000), 0x00);
    }

    #[test]
    fn test_handles_address() {
        let dsp = Dsp1::new();
        
        assert!(dsp.handles_address(0x6000));
        assert!(dsp.handles_address(0x6FFF));
        assert!(dsp.handles_address(0x7000));
        assert!(dsp.handles_address(0x7FFF));
        assert!(!dsp.handles_address(0x5FFF));
        assert!(!dsp.handles_address(0x8000));
    }
}
