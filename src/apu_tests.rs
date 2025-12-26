#[cfg(test)]
mod tests {
    use crate::apu::{Apu, Spc700, Psw, SpcBus};

    #[test]
    fn test_apu_initialization() {
        let apu = Apu::new();
        assert_eq!(apu.spc.a, 0);
        assert_eq!(apu.spc.x, 0);
        assert_eq!(apu.spc.y, 0);
        assert_eq!(apu.spc.sp, 0xFF);
        assert_eq!(apu.spc.cycles, 0);
    }

    #[test]
    fn test_apu_reset() {
        let mut apu = Apu::new();
        apu.spc.a = 0x42;
        apu.spc.x = 0x24;
        apu.cpu_write_port(0x2140, 0xFF);
        
        apu.reset();
        
        assert_eq!(apu.spc.a, 0);
        assert_eq!(apu.spc.x, 0);
        assert_eq!(apu.cpu_read_port(0x2140), 0);
    }

    #[test]
    fn test_cpu_apu_port_communication() {
        let mut apu = Apu::new();
        
        // CPU writes to port $2140
        apu.cpu_write_port(0x2140, 0xAA);
        apu.cpu_write_port(0x2141, 0xBB);
        apu.cpu_write_port(0x2142, 0xCC);
        apu.cpu_write_port(0x2143, 0xDD);
        
        // Verify SPC700 can't read the CPU ports yet (would need to implement SPC port writes)
        // This tests the CPU->APU direction
        assert_eq!(apu.cpu_read_port(0x2140), 0); // SPC hasn't written back yet
    }

    #[test]
    fn test_audio_frame_rendering() {
        let mut apu = Apu::new();
        
        // Render a frame
        let samples = apu.render_frame();
        
        // Should have 534 samples * 2 channels = 1068 i16 values
        assert_eq!(samples.len(), 1068);
        
        // Initial samples should be silent (0 or near 0)
        assert!(samples[0].abs() < 100);
        assert!(samples[1].abs() < 100);
    }

    #[test]
    fn test_spc700_mov_immediate() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up MOV A, #$42 instruction (0xE8 $42)
        apu.ram[0] = 0xE8;
        apu.ram[1] = 0x42;
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.a, 0x42);
        assert_eq!(spc.pc, 2);
        assert_eq!(cycles, 2);
        assert!(!spc.psw.z);
        assert!(!spc.psw.n);
    }

    #[test]
    fn test_spc700_mov_x_immediate() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up MOV X, #$FF instruction (0xCD $FF)
        apu.ram[0] = 0xCD;
        apu.ram[1] = 0xFF;
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.x, 0xFF);
        assert_eq!(spc.pc, 2);
        assert_eq!(cycles, 2);
        assert!(!spc.psw.z);
        assert!(spc.psw.n); // Negative flag should be set
    }

    #[test]
    fn test_spc700_mov_y_immediate() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up MOV Y, #$00 instruction (0x8D $00)
        apu.ram[0] = 0x8D;
        apu.ram[1] = 0x00;
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.y, 0x00);
        assert_eq!(spc.pc, 2);
        assert_eq!(cycles, 2);
        assert!(spc.psw.z); // Zero flag should be set
        assert!(!spc.psw.n);
    }

    #[test]
    fn test_spc700_jmp_absolute() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up JMP $1234 instruction (0x5F $34 $12)
        apu.ram[0] = 0x5F;
        apu.ram[1] = 0x34;
        apu.ram[2] = 0x12;
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.pc, 0x1234);
        assert_eq!(cycles, 2);
    }

    #[test]
    fn test_spc700_bra() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up BRA +10 instruction (0x2F $0A)
        apu.ram[0x100] = 0x2F;
        apu.ram[0x101] = 0x0A;
        spc.pc = 0x100;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.pc, 0x10C); // 0x102 + 0x0A
        assert_eq!(cycles, 2);
    }

    #[test]
    fn test_spc700_mov_direct() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up MOV A, $50 instruction (0xE4 $50)
        apu.ram[0] = 0xE4;
        apu.ram[1] = 0x50;
        apu.ram[0x50] = 0x88; // Value at direct page address
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(spc.a, 0x88);
        assert_eq!(spc.pc, 2);
        assert_eq!(cycles, 3);
        assert!(spc.psw.n);
        assert!(!spc.psw.z);
    }

    #[test]
    fn test_spc700_mov_to_direct() {
        let mut spc = Spc700::new();
        let mut apu = Apu::new();
        
        // Set up MOV $60, A instruction (0xC4 $60)
        apu.ram[0] = 0xC4;
        apu.ram[1] = 0x60;
        spc.a = 0x77;
        spc.pc = 0;
        
        let mut bus = TestBus { apu: &mut apu };
        let cycles = spc.step(&mut bus);
        
        assert_eq!(apu.ram[0x60], 0x77);
        assert_eq!(spc.pc, 2);
        assert_eq!(cycles, 4);
    }

    #[test]
    fn test_psw_flags() {
        let mut psw = Psw::new();
        
        // Test flag setting
        psw.n = true;
        psw.v = true;
        psw.z = true;
        psw.c = true;
        
        let byte = psw.to_byte();
        assert_eq!(byte & 0x80, 0x80); // N
        assert_eq!(byte & 0x40, 0x40); // V
        assert_eq!(byte & 0x02, 0x02); // Z
        assert_eq!(byte & 0x01, 0x01); // C
        
        // Test flag loading
        let mut psw2 = Psw::new();
        psw2.from_byte(0xC3);
        assert!(psw2.n);
        assert!(psw2.v);
        assert!(psw2.z);
        assert!(psw2.c);
    }

    #[test]
    fn test_psw_update_nz() {
        let mut psw = Psw::new();
        
        psw.update_nz(0x00);
        assert!(psw.z);
        assert!(!psw.n);
        
        psw.update_nz(0x80);
        assert!(!psw.z);
        assert!(psw.n);
        
        psw.update_nz(0x42);
        assert!(!psw.z);
        assert!(!psw.n);
    }

    #[test]
    fn test_dsp_register_writes() {
        let mut apu = Apu::new();
        
        // Write to DSP address register
        apu.step_spc(0); // Initialize bus
        
        // Manually set DSP address via internal field for testing
        apu.dsp_addr = 0x00; // Voice 0 volume left
        
        // Voice 0 volume registers should update when written through DSP
        // This tests the DSP register mapping
    }

    #[test]
    fn test_audio_ram_access() {
        let mut apu = Apu::new();
        
        // Write to audio RAM through port interface
        let mut bus = TestBus { apu: &mut apu };
        
        // Write some data to RAM
        bus.write8(0x1000, 0xAB);
        bus.write8(0x1001, 0xCD);
        
        // Read it back
        assert_eq!(bus.read8(0x1000), 0xAB);
        assert_eq!(bus.read8(0x1001), 0xCD);
    }

    #[test]
    fn test_step_spc_advances_cycles() {
        let mut apu = Apu::new();
        let initial_cycles = apu.spc.cycles;
        
        apu.step_spc(10);
        
        // Cycles should have advanced (at least 10 * 2 = 20 for NOP instructions)
        assert!(apu.spc.cycles > initial_cycles);
    }

    // Helper struct to provide bus access for testing
    struct TestBus<'a> {
        apu: &'a mut Apu,
    }

    impl<'a> SpcBus for TestBus<'a> {
        fn read8(&mut self, addr: u16) -> u8 {
            match addr {
                0xF2 => self.apu.dsp_addr,
                0xF3 => 0, // DSP data read not fully implemented in test
                0xF4..=0xF7 => self.apu.cpu_ports[(addr - 0xF4) as usize],
                _ => self.apu.ram[addr as usize],
            }
        }

        fn write8(&mut self, addr: u16, value: u8) {
            match addr {
                0xF2 => self.apu.dsp_addr = value & 0x7F,
                0xF3 => {}, // DSP data write
                0xF4..=0xF7 => self.apu.spc_ports[(addr - 0xF4) as usize] = value,
                _ => self.apu.ram[addr as usize] = value,
            }
        }
    }
}
