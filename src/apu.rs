//! Simplified SNES APU (SPC700 + DSP)
//! This is a staged, non-cycle-accurate implementation intended to provide
//! audio plumbing and basic tone generation while leaving room for future
//! accuracy improvements.

const AUDIO_RAM_SIZE: usize = 0x10000; // 64KB
const DSP_REGISTER_SPACE: usize = 0x80; // $00-$7F
pub const SAMPLE_RATE: u32 = 32_000;
pub const NTSC_SAMPLES_PER_FRAME: usize = 534; // ~32kHz over 60Hz

/// APU top-level structure holding the SPC700 core, DSP, and shared audio RAM.
pub struct Apu {
    pub spc: Spc700,
    pub(crate) ram: Box<[u8; AUDIO_RAM_SIZE]>,
    pub(crate) dsp: Dsp,
    pub(crate) cpu_ports: [u8; 4],
    pub(crate) spc_ports: [u8; 4],
    pub(crate) dsp_addr: u8,
    audio_buffer: Vec<i16>,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            spc: Spc700::new(),
            ram: Box::new([0; AUDIO_RAM_SIZE]),
            dsp: Dsp::new(),
            cpu_ports: [0; 4],
            spc_ports: [0; 4],
            dsp_addr: 0,
            audio_buffer: Vec::with_capacity(NTSC_SAMPLES_PER_FRAME * 2),
        }
    }

    pub fn reset(&mut self) {
        self.spc.reset();
        self.ram.fill(0);
        self.dsp.reset();
        self.cpu_ports = [0; 4];
        self.spc_ports = [0; 4];
        self.dsp_addr = 0;
        self.audio_buffer.clear();
    }

    /// Run the SPC700 for a small number of cycles. This is a placeholder that
    /// keeps the core alive without attempting exact timing.
    pub fn step_spc(&mut self, cycles: u32) {
        for _ in 0..cycles {
            // Split-borrow the APU so the SPC core can access RAM/DSP safely.
            let mut bus = ApuBusView {
                ram: &mut self.ram,
                dsp: &mut self.dsp,
                dsp_addr: &mut self.dsp_addr,
                cpu_ports: &self.cpu_ports,
                spc_ports: &mut self.spc_ports,
            };
            let _ = self.spc.step(&mut bus);
        }
    }

    /// Generate one NTSC frame worth of stereo samples at 32kHz.
    pub fn render_frame(&mut self) -> &[i16] {
        self.audio_buffer.clear();
        for _ in 0..NTSC_SAMPLES_PER_FRAME {
            let (l, r) = self.dsp.render_sample(&self.ram[..]);
            self.audio_buffer.push(l);
            self.audio_buffer.push(r);
        }
        &self.audio_buffer
    }

    /// CPU (65816) writes to APU I/O ports $2140-$2143.
    pub fn cpu_write_port(&mut self, addr: u16, value: u8) {
        let index = (addr & 0x3) as usize;
        self.cpu_ports[index] = value;
    }

    /// CPU (65816) reads from APU I/O ports $2140-$2143.
    pub fn cpu_read_port(&self, addr: u16) -> u8 {
        let index = (addr & 0x3) as usize;
        self.spc_ports[index]
    }

}

/// Simple SPC700 bus trait to decouple core from backing memory/DSP.
pub trait SpcBus {
    fn read8(&mut self, addr: u16) -> u8;
    fn write8(&mut self, addr: u16, value: u8);
}

struct ApuBusView<'a> {
    ram: &'a mut [u8; AUDIO_RAM_SIZE],
    dsp: &'a mut Dsp,
    dsp_addr: &'a mut u8,
    cpu_ports: &'a [u8; 4],
    spc_ports: &'a mut [u8; 4],
}

impl SpcBus for ApuBusView<'_> {
    fn read8(&mut self, addr: u16) -> u8 {
        match addr {
            0xF2 => *self.dsp_addr,
            0xF3 => self.dsp.read_register(*self.dsp_addr),
            0xF4..=0xF7 => self.cpu_ports[(addr - 0xF4) as usize],
            _ => self.ram[addr as usize],
        }
    }

    fn write8(&mut self, addr: u16, value: u8) {
        match addr {
            0xF2 => *self.dsp_addr = value & 0x7F,
            0xF3 => self.dsp.write_register(*self.dsp_addr, value, &self.ram[..]),
            0xF4..=0xF7 => self.spc_ports[(addr - 0xF4) as usize] = value,
            _ => self.ram[addr as usize] = value,
        }
    }
}

/// SPC700 CPU core (very small subset).
pub struct Spc700 {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub psw: Psw,
    pub sp: u8,
    pub pc: u16,
    pub cycles: u64,
}

impl Spc700 {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            psw: Psw::new(),
            sp: 0xFF,
            pc: 0,
            cycles: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.psw = Psw::new();
        self.sp = 0xFF;
        self.pc = 0xFFC0; // reset vector in ARAM will redirect on first step
        self.cycles = 0;
    }

    /// Execute a single opcode. This is intentionally incomplete; unsupported
    /// opcodes act as NOPs to keep the core running without panicking.
    pub fn step<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let opcode = bus.read8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let cycles = match opcode {
            0x00 => 2, // NOP placeholder
            0xE8 => self.mov_a_imm(bus),
            0xCD => self.mov_x_imm(bus),
            0x8D => self.mov_y_imm(bus),
            0xC4 => self.mov_direct_a(bus),
            0xE4 => self.mov_a_direct(bus),
            0x5F => self.jmp_abs(bus),
            0x2F => self.bra(bus),
            _ => 2, // Unknown opcode -> treat as NOP
        };
        self.cycles = self.cycles.wrapping_add(cycles as u64);
        cycles
    }

    fn fetch8<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let v = bus.read8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16<B: SpcBus>(&mut self, bus: &mut B) -> u16 {
        let lo = self.fetch8(bus) as u16;
        let hi = self.fetch8(bus) as u16;
        (hi << 8) | lo
    }

    fn mov_a_imm<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let value = self.fetch8(bus);
        self.a = value;
        self.psw.update_nz(self.a);
        2
    }

    fn mov_x_imm<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let value = self.fetch8(bus);
        self.x = value;
        self.psw.update_nz(self.x);
        2
    }

    fn mov_y_imm<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let value = self.fetch8(bus);
        self.y = value;
        self.psw.update_nz(self.y);
        2
    }

    fn mov_a_direct<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let addr = self.fetch8(bus) as u16;
        self.a = bus.read8(addr);
        self.psw.update_nz(self.a);
        3
    }

    fn mov_direct_a<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let addr = self.fetch8(bus) as u16;
        bus.write8(addr, self.a);
        4
    }

    fn jmp_abs<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let addr = self.fetch16(bus);
        self.pc = addr;
        2
    }

    fn bra<B: SpcBus>(&mut self, bus: &mut B) -> u8 {
        let offset = self.fetch8(bus) as i8;
        self.pc = self.pc.wrapping_add(offset as u16);
        2
    }
}

/// Processor Status Word
#[derive(Clone, Copy)]
pub struct Psw {
    pub n: bool,
    pub v: bool,
    pub p: bool,
    pub b: bool,
    pub h: bool,
    pub i: bool,
    pub z: bool,
    pub c: bool,
}

impl Psw {
    pub fn new() -> Self {
        Self {
            n: false,
            v: false,
            p: false,
            b: false,
            h: false,
            i: false,
            z: false,
            c: false,
        }
    }

    pub fn to_byte(&self) -> u8 {
        (self.n as u8) << 7
            | (self.v as u8) << 6
            | (self.p as u8) << 5
            | (self.b as u8) << 4
            | (self.h as u8) << 3
            | (self.i as u8) << 2
            | (self.z as u8) << 1
            | (self.c as u8)
    }

    pub fn from_byte(&mut self, value: u8) {
        self.n = (value & 0x80) != 0;
        self.v = (value & 0x40) != 0;
        self.p = (value & 0x20) != 0;
        self.b = (value & 0x10) != 0;
        self.h = (value & 0x08) != 0;
        self.i = (value & 0x04) != 0;
        self.z = (value & 0x02) != 0;
        self.c = (value & 0x01) != 0;
    }

    pub fn update_nz(&mut self, value: u8) {
        self.n = (value & 0x80) != 0;
        self.z = value == 0;
    }
}

/// DSP audio unit (very approximate, focuses on plumbing and basic tone).
pub(crate) struct Dsp {
    registers: [u8; DSP_REGISTER_SPACE],
    voices: [Voice; 8],
    echo_buffer: Vec<f32>,
    echo_pos: usize,
}

impl Dsp {
    fn new() -> Self {
        Self {
            registers: [0; DSP_REGISTER_SPACE],
            voices: [Voice::new(); 8],
            echo_buffer: vec![0.0; 2048],
            echo_pos: 0,
        }
    }

    fn reset(&mut self) {
        self.registers = [0; DSP_REGISTER_SPACE];
        for voice in &mut self.voices {
            *voice = Voice::new();
        }
        self.echo_buffer.fill(0.0);
        self.echo_pos = 0;
    }

    fn read_register(&self, addr: u8) -> u8 {
        self.registers[(addr as usize) % DSP_REGISTER_SPACE]
    }

    fn write_register(&mut self, addr: u8, value: u8, ram: &[u8]) {
        let index = (addr as usize) % DSP_REGISTER_SPACE;
        self.registers[index] = value;

        // Update voice parameters when mapped registers change.
        let voice_index = (index / 0x10) % 8;
        let v = &mut self.voices[voice_index];
        match index % 0x10 {
            0x0 => v.volume_l = value as i16,
            0x1 => v.volume_r = value as i16,
            0x2 => v.pitch = (v.pitch & 0xFF00) | value as u16,
            0x3 => v.pitch = (v.pitch & 0x00FF) | ((value as u16) << 8),
            0x4 => v.srcn = value,
            0x5 => v.adsr1 = value,
            0x6 => v.adsr2 = value,
            0x7 => v.gain = value,
            _ => {}
        }

        // Key on/off handling uses shared registers 0x4C/0x5C.
        match index {
            0x4C => self.handle_key_on(value, ram),
            0x5C => self.handle_key_off(value),
            _ => {}
        }
    }

    fn handle_key_on(&mut self, mask: u8, ram: &[u8]) {
        for i in 0..8 {
            if (mask & (1 << i)) != 0 {
                let srcn = self.voices[i].srcn;
                let start_addr = (srcn as u16) << 8;
                self.voices[i].trigger(start_addr, ram);
            }
        }
    }

    fn handle_key_off(&mut self, mask: u8) {
        for i in 0..8 {
            if (mask & (1 << i)) != 0 {
                self.voices[i].active = false;
            }
        }
    }

    fn render_sample(&mut self, ram: &[u8]) -> (i16, i16) {
        let mut mix_l = 0.0f32;
        let mut mix_r = 0.0f32;
        let mut last_sample = 0.0f32;
        let pitch_mod_mask = self.registers[0x2D];
        let mut mod_flags = [false; 8];
        for i in 0..7 {
            mod_flags[i + 1] = (pitch_mod_mask & (1 << i)) != 0;
        }

        for (i, voice) in self.voices.iter_mut().enumerate() {
            voice.pitch_mod = mod_flags[i];
            let sample = voice.next_sample(ram, last_sample);
            last_sample = sample;
            mix_l += sample * (voice.volume_l as f32 / 127.0);
            mix_r += sample * (voice.volume_r as f32 / 127.0);
        }

        // Simple echo/reverb: feed-forward + feedback delay line.
        let echo = self.echo_buffer[self.echo_pos];
        let fb = (self.registers[0x0D] as f32) / 127.0; // EFB
        let evl = (self.registers[0x2C] as f32) / 127.0; // EVOL(L)
        let evr = (self.registers[0x3C] as f32) / 127.0; // EVOL(R)
        let out_l = mix_l + echo * evl;
        let out_r = mix_r + echo * evr;

        let echo_input = (out_l + out_r) * 0.5 + echo * fb;
        self.echo_buffer[self.echo_pos] = echo_input;
        self.echo_pos = (self.echo_pos + 1) % self.echo_buffer.len();

        (clamp_i16(out_l), clamp_i16(out_r))
    }
}

#[derive(Clone, Copy)]
struct Voice {
    volume_l: i16,
    volume_r: i16,
    pitch: u16,
    srcn: u8,
    adsr1: u8,
    adsr2: u8,
    gain: u8,
    env_level: f32,
    brr_addr: u16,
    brr_offset: usize,
    decoded: [i16; 16],
    decoded_index: usize,
    phase: f32,
    active: bool,
    pitch_mod: bool,
}

impl Voice {
    const fn new() -> Self {
        Self {
            volume_l: 0,
            volume_r: 0,
            pitch: 0,
            srcn: 0,
            adsr1: 0,
            adsr2: 0,
            gain: 0,
            env_level: 0.0,
            brr_addr: 0,
            brr_offset: 0,
            decoded: [0; 16],
            decoded_index: 16,
            phase: 0.0,
            active: false,
            pitch_mod: false,
        }
    }

    fn trigger(&mut self, start_addr: u16, ram: &[u8]) {
        self.env_level = 0.0;
        self.active = true;
        self.brr_addr = start_addr;
        self.brr_offset = 0;
        self.decoded_index = 16; // Force decode on first sample
        self.phase = 0.0;
        self.decode_next_block(ram);
    }

    fn next_sample(&mut self, ram: &[u8], pitch_mod_input: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        let pitch_base = self.pitch as f32 / 4096.0;
        let pitch_delta = if self.pitch_mod { pitch_mod_input * 0.0005 } else { 0.0 };
        self.phase += pitch_base + pitch_delta;

        while self.phase >= 1.0 {
            self.phase -= 1.0;
            self.decoded_index += 1;
            if self.decoded_index >= 16 {
                self.decode_next_block(ram);
                self.decoded_index = 0;
            }
        }

        let sample = self.decoded.get(self.decoded_index).copied().unwrap_or(0) as f32;
        let env = self.advance_envelope();
        sample * env / 32768.0
    }

    fn advance_envelope(&mut self) -> f32 {
        // Very rough ADSR approximation to keep dynamics alive.
        let attack_rate = ((self.adsr1 >> 4) & 0x0F) as f32 / 15.0 * 0.005;
        let decay_rate = ((self.adsr2 >> 4) & 0x07) as f32 / 7.0 * 0.002;
        let sustain = (self.adsr2 & 0x1F) as f32 / 31.0;

        if self.env_level < 1.0 {
            self.env_level = (self.env_level + attack_rate).min(1.0);
        } else if self.env_level > sustain {
            self.env_level = (self.env_level - decay_rate).max(sustain);
        }

        // GAIN acts as simple direct level when ADSR is disabled.
        if (self.adsr1 & 0x80) == 0 {
            self.env_level = (self.gain as f32) / 255.0;
        }

        self.env_level
    }

    fn decode_next_block(&mut self, ram: &[u8]) {
        let addr = self.brr_addr as usize + self.brr_offset;
        if addr + 9 > ram.len() {
            self.active = false;
            return;
        }
        let header = ram[addr];
        let data = &ram[addr + 1..addr + 9];
        self.decoded = decode_brr_block(header, data);

        self.brr_offset += 9;
        if (header & 0x01) != 0 {
            // End flag; stop voice after block completes.
            self.active = false;
        }
    }
}

fn decode_brr_block(header: u8, data: &[u8]) -> [i16; 16] {
    let range = (header >> 4) & 0x0F;
    let filter = (header >> 2) & 0x03;
    let mut output = [0i16; 16];
    let mut prev1: i32 = 0;
    let mut prev2: i32 = 0;

    for i in 0..16 {
        let byte = data[i / 2];
        let nybble = if i % 2 == 0 { byte >> 4 } else { byte & 0x0F };
        let mut sample = ((nybble as i8) << 4) as i32;
        sample >>= range;

        // Filters from BRR spec
        sample = match filter {
            0 => sample,
            1 => sample + (prev1 * 15 / 16),
            2 => sample + (prev1 * 61 / 32) - (prev2 * 15 / 16),
            3 => sample + (prev1 * 115 / 64) - (prev2 * 13 / 16),
            _ => sample,
        };

        sample = sample.clamp(-32768, 32767);
        output[i] = sample as i16;
        prev2 = prev1;
        prev1 = sample;
    }

    output
}

fn clamp_i16(v: f32) -> i16 {
    v.clamp(-32768.0, 32767.0) as i16
}
