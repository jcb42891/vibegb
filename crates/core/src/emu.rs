use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const DIV_ADDR: u16 = 0xFF04;
pub const TIMA_ADDR: u16 = 0xFF05;
pub const TMA_ADDR: u16 = 0xFF06;
pub const TAC_ADDR: u16 = 0xFF07;
pub const IF_ADDR: u16 = 0xFF0F;
pub const IE_ADDR: u16 = 0xFFFF;
pub const SB_ADDR: u16 = 0xFF01;
pub const SC_ADDR: u16 = 0xFF02;

pub const INTERRUPT_VBLANK: u8 = 0x01;
pub const INTERRUPT_LCD: u8 = 0x02;
pub const INTERRUPT_TIMER: u8 = 0x04;
pub const INTERRUPT_SERIAL: u8 = 0x08;
pub const INTERRUPT_JOYPAD: u8 = 0x10;

const FLAG_Z: u8 = 0x80;
const FLAG_N: u8 = 0x40;
const FLAG_H: u8 = 0x20;
const FLAG_C: u8 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuError {
    IllegalOpcode(u8),
}

impl Display for EmuError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IllegalOpcode(opcode) => write!(f, "illegal opcode 0x{opcode:02X}"),
        }
    }
}

impl Error for EmuError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
}

impl Registers {
    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f & 0xF0])
    }

    pub fn set_af(&mut self, value: u16) {
        let [a, f] = value.to_be_bytes();
        self.a = a;
        self.f = f & 0xF0;
    }

    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    pub fn set_bc(&mut self, value: u16) {
        let [b, c] = value.to_be_bytes();
        self.b = b;
        self.c = c;
    }

    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    pub fn set_de(&mut self, value: u16) {
        let [d, e] = value.to_be_bytes();
        self.d = d;
        self.e = e;
    }

    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_hl(&mut self, value: u16) {
        let [h, l] = value.to_be_bytes();
        self.h = h;
        self.l = l;
    }

    pub fn flag_z(&self) -> bool {
        self.f & FLAG_Z != 0
    }

    pub fn flag_n(&self) -> bool {
        self.f & FLAG_N != 0
    }

    pub fn flag_h(&self) -> bool {
        self.f & FLAG_H != 0
    }

    pub fn flag_c(&self) -> bool {
        self.f & FLAG_C != 0
    }

    pub fn set_z(&mut self, value: bool) {
        self.set_flag(FLAG_Z, value);
    }

    pub fn set_n(&mut self, value: bool) {
        self.set_flag(FLAG_N, value);
    }

    pub fn set_h(&mut self, value: bool) {
        self.set_flag(FLAG_H, value);
    }

    pub fn set_c(&mut self, value: bool) {
        self.set_flag(FLAG_C, value);
    }

    fn set_flag(&mut self, mask: u8, value: bool) {
        if value {
            self.f |= mask;
        } else {
            self.f &= !mask;
        }
        self.f &= 0xF0;
    }
}

#[derive(Debug, Clone, Default)]
struct Timer {
    divider: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    overflow_reload_delay: Option<u8>,
}

impl Timer {
    fn div(&self) -> u8 {
        (self.divider >> 8) as u8
    }

    fn tac_read(&self) -> u8 {
        0xF8 | (self.tac & 0x07)
    }

    fn write_div(&mut self) {
        let previous_input = self.timer_input(self.divider);
        self.divider = 0;
        let next_input = self.timer_input(self.divider);
        if previous_input && !next_input {
            self.increment_tima();
        }
    }

    fn write_tima(&mut self, value: u8) {
        self.tima = value;
        self.overflow_reload_delay = None;
    }

    fn write_tma(&mut self, value: u8) {
        self.tma = value;
    }

    fn write_tac(&mut self, value: u8) {
        let previous_input = self.timer_input(self.divider);
        self.tac = value & 0x07;
        let next_input = self.timer_input(self.divider);
        if previous_input && !next_input {
            self.increment_tima();
        }
    }

    fn tick(&mut self, cycles: u32, interrupt_flags: &mut u8) {
        for _ in 0..cycles {
            self.tick_one(interrupt_flags);
        }
    }

    fn tick_one(&mut self, interrupt_flags: &mut u8) {
        let previous_input = self.timer_input(self.divider);
        self.divider = self.divider.wrapping_add(1);
        let next_input = self.timer_input(self.divider);
        if previous_input && !next_input {
            self.increment_tima();
        }
        self.handle_reload(interrupt_flags);
    }

    fn handle_reload(&mut self, interrupt_flags: &mut u8) {
        if let Some(delay) = self.overflow_reload_delay {
            if delay == 0 {
                if self.tima == 0 {
                    self.tima = self.tma;
                    *interrupt_flags |= INTERRUPT_TIMER;
                }
                self.overflow_reload_delay = None;
            } else {
                self.overflow_reload_delay = Some(delay - 1);
            }
        }
    }

    fn increment_tima(&mut self) {
        if self.tima == 0xFF {
            self.tima = 0x00;
            self.overflow_reload_delay = Some(4);
        } else {
            self.tima = self.tima.wrapping_add(1);
        }
    }

    fn selected_bit(&self) -> u16 {
        match self.tac & 0x03 {
            0 => 9,
            1 => 3,
            2 => 5,
            3 => 7,
            _ => unreachable!(),
        }
    }

    fn timer_input(&self, divider: u16) -> bool {
        (self.tac & 0x04) != 0 && (divider & (1u16 << self.selected_bit())) != 0
    }
}

#[derive(Debug, Clone)]
pub struct Bus {
    memory: [u8; 0x10000],
    timer: Timer,
    interrupt_enable: u8,
    interrupt_flags: u8,
    serial_output: Vec<u8>,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            memory: [0; 0x10000],
            timer: Timer::default(),
            interrupt_enable: 0,
            interrupt_flags: 0,
            serial_output: Vec::new(),
        }
    }
}

impl Bus {
    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            DIV_ADDR => self.timer.div(),
            TIMA_ADDR => self.timer.tima,
            TMA_ADDR => self.timer.tma,
            TAC_ADDR => self.timer.tac_read(),
            IF_ADDR => 0xE0 | (self.interrupt_flags & 0x1F),
            IE_ADDR => self.interrupt_enable & 0x1F,
            _ => self.memory[address as usize],
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            DIV_ADDR => self.timer.write_div(),
            TIMA_ADDR => self.timer.write_tima(value),
            TMA_ADDR => self.timer.write_tma(value),
            TAC_ADDR => self.timer.write_tac(value),
            IF_ADDR => self.interrupt_flags = value & 0x1F,
            IE_ADDR => self.interrupt_enable = value & 0x1F,
            SB_ADDR => self.memory[SB_ADDR as usize] = value,
            SC_ADDR => {
                self.memory[SC_ADDR as usize] = value;
                if value & 0x81 == 0x81 {
                    self.serial_output.push(self.memory[SB_ADDR as usize]);
                    self.memory[SC_ADDR as usize] = value & !0x80;
                }
            }
            _ => {
                self.memory[address as usize] = value;
            }
        }
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let lo = self.read_byte(address);
        let hi = self.read_byte(address.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    pub fn write_word(&mut self, address: u16, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.write_byte(address, lo);
        self.write_byte(address.wrapping_add(1), hi);
    }

    pub fn load_bytes(&mut self, start: u16, data: &[u8]) {
        let start = start as usize;
        if start >= self.memory.len() {
            return;
        }
        let max = min(data.len(), self.memory.len() - start);
        self.memory[start..start + max].copy_from_slice(&data[..max]);
    }

    pub fn tick(&mut self, cycles: u32) {
        self.timer.tick(cycles, &mut self.interrupt_flags);
    }

    pub fn pending_interrupts(&self) -> u8 {
        self.interrupt_enable & self.interrupt_flags & 0x1F
    }

    pub fn request_interrupt(&mut self, mask: u8) {
        self.interrupt_flags |= mask & 0x1F;
    }

    pub fn clear_interrupt(&mut self, mask: u8) {
        self.interrupt_flags &= !(mask & 0x1F);
    }

    pub fn serial_output(&self) -> &[u8] {
        &self.serial_output
    }

    pub fn take_serial_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.serial_output)
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_program(start: u16, program: &[u8]) -> Self {
        let mut gb = Self::default();
        gb.cpu.pc = start;
        gb.bus.load_bytes(start, program);
        gb
    }

    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.bus.load_bytes(0x0000, rom_data);
        self.cpu.pc = 0x0100;
        self.cpu.sp = 0xFFFE;
    }

    pub fn step(&mut self) -> Result<u32, EmuError> {
        self.cpu.step(&mut self.bus)
    }

    pub fn run_steps(&mut self, steps: usize) -> Result<u64, EmuError> {
        let mut cycles = 0u64;
        for _ in 0..steps {
            cycles += u64::from(self.step()?);
        }
        Ok(cycles)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    pub regs: Registers,
    pub pc: u16,
    pub sp: u16,
    pub ime: bool,
    pub halted: bool,
    pub stopped: bool,
    ime_delay: u8,
    halt_bug: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            regs: Registers::default(),
            pc: 0x0000,
            sp: 0xFFFE,
            ime: false,
            halted: false,
            stopped: false,
            ime_delay: 0,
            halt_bug: false,
        }
    }
}

impl Cpu {
    pub fn step(&mut self, bus: &mut Bus) -> Result<u32, EmuError> {
        if self.stopped {
            if bus.pending_interrupts() != 0 {
                self.stopped = false;
            } else {
                bus.tick(4);
                return Ok(4);
            }
        }

        let pending = bus.pending_interrupts();
        if self.ime && pending != 0 {
            let cycles = self.service_interrupt(bus);
            bus.tick(cycles);
            return Ok(cycles);
        }

        if self.halted {
            if pending != 0 {
                self.halted = false;
            } else {
                bus.tick(4);
                return Ok(4);
            }
        }

        let opcode = self.fetch_byte(bus);
        let cycles = self.execute_base(opcode, bus)?;
        bus.tick(cycles);
        self.advance_ime_delay();
        Ok(cycles)
    }

    fn execute_base(&mut self, opcode: u8, bus: &mut Bus) -> Result<u32, EmuError> {
        match opcode {
            0x00 => Ok(4),
            0x01 | 0x11 | 0x21 | 0x31 => {
                let value = self.fetch_word(bus);
                match opcode {
                    0x01 => self.regs.set_bc(value),
                    0x11 => self.regs.set_de(value),
                    0x21 => self.regs.set_hl(value),
                    0x31 => self.sp = value,
                    _ => unreachable!(),
                }
                Ok(12)
            }
            0x02 => {
                bus.write_byte(self.regs.bc(), self.regs.a);
                Ok(8)
            }
            0x12 => {
                bus.write_byte(self.regs.de(), self.regs.a);
                Ok(8)
            }
            0x22 => {
                let hl = self.regs.hl();
                bus.write_byte(hl, self.regs.a);
                self.regs.set_hl(hl.wrapping_add(1));
                Ok(8)
            }
            0x32 => {
                let hl = self.regs.hl();
                bus.write_byte(hl, self.regs.a);
                self.regs.set_hl(hl.wrapping_sub(1));
                Ok(8)
            }
            0x03 | 0x13 | 0x23 | 0x33 => {
                match opcode {
                    0x03 => self.regs.set_bc(self.regs.bc().wrapping_add(1)),
                    0x13 => self.regs.set_de(self.regs.de().wrapping_add(1)),
                    0x23 => self.regs.set_hl(self.regs.hl().wrapping_add(1)),
                    0x33 => self.sp = self.sp.wrapping_add(1),
                    _ => unreachable!(),
                }
                Ok(8)
            }
            op if op & 0b1100_0111 == 0b0000_0100 => {
                let register = (op >> 3) & 0x07;
                let value = self.read_r8(bus, register);
                let result = self.inc8(value);
                self.write_r8(bus, register, result);
                Ok(if register == 6 { 12 } else { 4 })
            }
            op if op & 0b1100_0111 == 0b0000_0101 => {
                let register = (op >> 3) & 0x07;
                let value = self.read_r8(bus, register);
                let result = self.dec8(value);
                self.write_r8(bus, register, result);
                Ok(if register == 6 { 12 } else { 4 })
            }
            op if op & 0b1100_0111 == 0b0000_0110 => {
                let register = (op >> 3) & 0x07;
                let value = self.fetch_byte(bus);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 12 } else { 8 })
            }
            0x07 => {
                self.regs.a = self.rlc(self.regs.a, false);
                Ok(4)
            }
            0x08 => {
                let address = self.fetch_word(bus);
                bus.write_word(address, self.sp);
                Ok(20)
            }
            0x09 | 0x19 | 0x29 | 0x39 => {
                let value = match opcode {
                    0x09 => self.regs.bc(),
                    0x19 => self.regs.de(),
                    0x29 => self.regs.hl(),
                    0x39 => self.sp,
                    _ => unreachable!(),
                };
                self.add_hl(value);
                Ok(8)
            }
            0x0A => {
                self.regs.a = bus.read_byte(self.regs.bc());
                Ok(8)
            }
            0x1A => {
                self.regs.a = bus.read_byte(self.regs.de());
                Ok(8)
            }
            0x2A => {
                let hl = self.regs.hl();
                self.regs.a = bus.read_byte(hl);
                self.regs.set_hl(hl.wrapping_add(1));
                Ok(8)
            }
            0x3A => {
                let hl = self.regs.hl();
                self.regs.a = bus.read_byte(hl);
                self.regs.set_hl(hl.wrapping_sub(1));
                Ok(8)
            }
            0x0B | 0x1B | 0x2B | 0x3B => {
                match opcode {
                    0x0B => self.regs.set_bc(self.regs.bc().wrapping_sub(1)),
                    0x1B => self.regs.set_de(self.regs.de().wrapping_sub(1)),
                    0x2B => self.regs.set_hl(self.regs.hl().wrapping_sub(1)),
                    0x3B => self.sp = self.sp.wrapping_sub(1),
                    _ => unreachable!(),
                }
                Ok(8)
            }
            0x0F => {
                self.regs.a = self.rrc(self.regs.a, false);
                Ok(4)
            }
            0x10 => {
                let _padding = self.fetch_byte(bus);
                self.stopped = true;
                Ok(4)
            }
            0x17 => {
                self.regs.a = self.rl(self.regs.a, false);
                Ok(4)
            }
            0x18 => {
                let offset = self.fetch_byte(bus) as i8;
                self.pc = Self::add_signed_u16(self.pc, offset);
                Ok(12)
            }
            op if matches!(op, 0x20 | 0x28 | 0x30 | 0x38) => {
                let offset = self.fetch_byte(bus) as i8;
                let condition = self.condition((op >> 3) & 0x03);
                if condition {
                    self.pc = Self::add_signed_u16(self.pc, offset);
                    Ok(12)
                } else {
                    Ok(8)
                }
            }
            0x1F => {
                self.regs.a = self.rr(self.regs.a, false);
                Ok(4)
            }
            0x27 => {
                self.daa();
                Ok(4)
            }
            0x2F => {
                self.regs.a = !self.regs.a;
                self.regs.set_n(true);
                self.regs.set_h(true);
                Ok(4)
            }
            0x37 => {
                self.regs.set_n(false);
                self.regs.set_h(false);
                self.regs.set_c(true);
                Ok(4)
            }
            0x3F => {
                let carry = self.regs.flag_c();
                self.regs.set_n(false);
                self.regs.set_h(false);
                self.regs.set_c(!carry);
                Ok(4)
            }
            0x40..=0x7F => {
                if opcode == 0x76 {
                    if self.ime {
                        self.halted = true;
                    } else if bus.pending_interrupts() != 0 {
                        self.halt_bug = true;
                    } else {
                        self.halted = true;
                    }
                    Ok(4)
                } else {
                    let destination = (opcode >> 3) & 0x07;
                    let source = opcode & 0x07;
                    let value = self.read_r8(bus, source);
                    self.write_r8(bus, destination, value);
                    Ok(if source == 6 || destination == 6 {
                        8
                    } else {
                        4
                    })
                }
            }
            0x80..=0xBF => {
                let source = opcode & 0x07;
                let value = self.read_r8(bus, source);
                match (opcode >> 3) & 0x07 {
                    0x00 => self.add_a(value, false),
                    0x01 => self.add_a(value, true),
                    0x02 => self.sub_a(value, false),
                    0x03 => self.sub_a(value, true),
                    0x04 => self.and_a(value),
                    0x05 => self.xor_a(value),
                    0x06 => self.or_a(value),
                    0x07 => self.cp_a(value),
                    _ => unreachable!(),
                }
                Ok(if source == 6 { 8 } else { 4 })
            }
            op if matches!(op, 0xC0 | 0xC8 | 0xD0 | 0xD8) => {
                let condition = self.condition((op >> 3) & 0x03);
                if condition {
                    self.pc = self.pop_word(bus);
                    Ok(20)
                } else {
                    Ok(8)
                }
            }
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let value = self.pop_word(bus);
                match opcode {
                    0xC1 => self.regs.set_bc(value),
                    0xD1 => self.regs.set_de(value),
                    0xE1 => self.regs.set_hl(value),
                    0xF1 => self.regs.set_af(value),
                    _ => unreachable!(),
                }
                Ok(12)
            }
            op if matches!(op, 0xC2 | 0xCA | 0xD2 | 0xDA) => {
                let address = self.fetch_word(bus);
                let condition = self.condition((op >> 3) & 0x03);
                if condition {
                    self.pc = address;
                    Ok(16)
                } else {
                    Ok(12)
                }
            }
            0xC3 => {
                self.pc = self.fetch_word(bus);
                Ok(16)
            }
            op if matches!(op, 0xC4 | 0xCC | 0xD4 | 0xDC) => {
                let address = self.fetch_word(bus);
                let condition = self.condition((op >> 3) & 0x03);
                if condition {
                    self.push_word(bus, self.pc);
                    self.pc = address;
                    Ok(24)
                } else {
                    Ok(12)
                }
            }
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let value = match opcode {
                    0xC5 => self.regs.bc(),
                    0xD5 => self.regs.de(),
                    0xE5 => self.regs.hl(),
                    0xF5 => self.regs.af(),
                    _ => unreachable!(),
                };
                self.push_word(bus, value);
                Ok(16)
            }
            0xC6 => {
                let value = self.fetch_byte(bus);
                self.add_a(value, false);
                Ok(8)
            }
            op if op & 0xC7 == 0xC7 => {
                let vector = u16::from(op & 0x38);
                self.push_word(bus, self.pc);
                self.pc = vector;
                Ok(16)
            }
            0xC9 => {
                self.pc = self.pop_word(bus);
                Ok(16)
            }
            0xCB => {
                let cb_opcode = self.fetch_byte(bus);
                self.execute_cb(cb_opcode, bus)
            }
            0xCD => {
                let address = self.fetch_word(bus);
                self.push_word(bus, self.pc);
                self.pc = address;
                Ok(24)
            }
            0xCE => {
                let value = self.fetch_byte(bus);
                self.add_a(value, true);
                Ok(8)
            }
            0xD6 => {
                let value = self.fetch_byte(bus);
                self.sub_a(value, false);
                Ok(8)
            }
            0xD9 => {
                self.pc = self.pop_word(bus);
                self.ime = true;
                self.ime_delay = 0;
                Ok(16)
            }
            0xDE => {
                let value = self.fetch_byte(bus);
                self.sub_a(value, true);
                Ok(8)
            }
            0xE0 => {
                let offset = self.fetch_byte(bus);
                let address = 0xFF00 | u16::from(offset);
                bus.write_byte(address, self.regs.a);
                Ok(12)
            }
            0xE2 => {
                let address = 0xFF00 | u16::from(self.regs.c);
                bus.write_byte(address, self.regs.a);
                Ok(8)
            }
            0xE6 => {
                let value = self.fetch_byte(bus);
                self.and_a(value);
                Ok(8)
            }
            0xE8 => {
                let offset = self.fetch_byte(bus);
                let (result, half_carry, carry) = Self::add_sp_offset(self.sp, offset);
                self.sp = result;
                self.regs.set_z(false);
                self.regs.set_n(false);
                self.regs.set_h(half_carry);
                self.regs.set_c(carry);
                Ok(16)
            }
            0xE9 => {
                self.pc = self.regs.hl();
                Ok(4)
            }
            0xEA => {
                let address = self.fetch_word(bus);
                bus.write_byte(address, self.regs.a);
                Ok(16)
            }
            0xEE => {
                let value = self.fetch_byte(bus);
                self.xor_a(value);
                Ok(8)
            }
            0xF0 => {
                let offset = self.fetch_byte(bus);
                let address = 0xFF00 | u16::from(offset);
                self.regs.a = bus.read_byte(address);
                Ok(12)
            }
            0xF2 => {
                let address = 0xFF00 | u16::from(self.regs.c);
                self.regs.a = bus.read_byte(address);
                Ok(8)
            }
            0xF3 => {
                self.ime = false;
                self.ime_delay = 0;
                Ok(4)
            }
            0xF6 => {
                let value = self.fetch_byte(bus);
                self.or_a(value);
                Ok(8)
            }
            0xF8 => {
                let offset = self.fetch_byte(bus);
                let (result, half_carry, carry) = Self::add_sp_offset(self.sp, offset);
                self.regs.set_hl(result);
                self.regs.set_z(false);
                self.regs.set_n(false);
                self.regs.set_h(half_carry);
                self.regs.set_c(carry);
                Ok(12)
            }
            0xF9 => {
                self.sp = self.regs.hl();
                Ok(8)
            }
            0xFA => {
                let address = self.fetch_word(bus);
                self.regs.a = bus.read_byte(address);
                Ok(16)
            }
            0xFB => {
                self.ime_delay = 2;
                Ok(4)
            }
            0xFE => {
                let value = self.fetch_byte(bus);
                self.cp_a(value);
                Ok(8)
            }
            0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
                Err(EmuError::IllegalOpcode(opcode))
            }
            _ => Err(EmuError::IllegalOpcode(opcode)),
        }
    }

    fn execute_cb(&mut self, opcode: u8, bus: &mut Bus) -> Result<u32, EmuError> {
        let register = opcode & 0x07;
        let mut value = self.read_r8(bus, register);

        match opcode {
            0x00..=0x07 => {
                value = self.rlc(value, true);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x08..=0x0F => {
                value = self.rrc(value, true);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x10..=0x17 => {
                value = self.rl(value, true);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x18..=0x1F => {
                value = self.rr(value, true);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x20..=0x27 => {
                value = self.sla(value);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x28..=0x2F => {
                value = self.sra(value);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x30..=0x37 => {
                value = self.swap(value);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x38..=0x3F => {
                value = self.srl(value);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0x40..=0x7F => {
                let bit = (opcode >> 3) & 0x07;
                self.regs.set_z((value & (1 << bit)) == 0);
                self.regs.set_n(false);
                self.regs.set_h(true);
                Ok(if register == 6 { 12 } else { 8 })
            }
            0x80..=0xBF => {
                let bit = (opcode >> 3) & 0x07;
                value &= !(1 << bit);
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
            0xC0..=0xFF => {
                let bit = (opcode >> 3) & 0x07;
                value |= 1 << bit;
                self.write_r8(bus, register, value);
                Ok(if register == 6 { 16 } else { 8 })
            }
        }
    }

    fn service_interrupt(&mut self, bus: &mut Bus) -> u32 {
        self.ime = false;
        self.ime_delay = 0;
        self.halted = false;
        self.stopped = false;

        let [pc_lo, pc_hi] = self.pc.to_le_bytes();

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, pc_hi);

        let pending_after_hi = bus.pending_interrupts();
        if pending_after_hi == 0 {
            self.sp = self.sp.wrapping_sub(1);
            bus.write_byte(self.sp, pc_lo);
            self.pc = 0x0000;
            return 20;
        }

        let (mask, vector) = Self::interrupt_vector(pending_after_hi);

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, pc_lo);

        bus.clear_interrupt(mask);
        self.pc = vector;
        20
    }

    fn interrupt_vector(pending: u8) -> (u8, u16) {
        if pending & INTERRUPT_VBLANK != 0 {
            (INTERRUPT_VBLANK, 0x40)
        } else if pending & INTERRUPT_LCD != 0 {
            (INTERRUPT_LCD, 0x48)
        } else if pending & INTERRUPT_TIMER != 0 {
            (INTERRUPT_TIMER, 0x50)
        } else if pending & INTERRUPT_SERIAL != 0 {
            (INTERRUPT_SERIAL, 0x58)
        } else {
            (INTERRUPT_JOYPAD, 0x60)
        }
    }

    fn advance_ime_delay(&mut self) {
        if self.ime_delay > 0 {
            self.ime_delay -= 1;
            if self.ime_delay == 0 {
                self.ime = true;
            }
        }
    }

    fn condition(&self, code: u8) -> bool {
        match code & 0x03 {
            0 => !self.regs.flag_z(),
            1 => self.regs.flag_z(),
            2 => !self.regs.flag_c(),
            3 => self.regs.flag_c(),
            _ => unreachable!(),
        }
    }

    fn fetch_byte(&mut self, bus: &Bus) -> u8 {
        if self.halt_bug {
            self.halt_bug = false;
            bus.read_byte(self.pc)
        } else {
            let byte = bus.read_byte(self.pc);
            self.pc = self.pc.wrapping_add(1);
            byte
        }
    }

    fn fetch_word(&mut self, bus: &Bus) -> u16 {
        let lo = self.fetch_byte(bus);
        let hi = self.fetch_byte(bus);
        u16::from_le_bytes([lo, hi])
    }

    fn push_word(&mut self, bus: &mut Bus, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, lo);
    }

    fn pop_word(&mut self, bus: &mut Bus) -> u16 {
        let lo = bus.read_byte(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read_byte(self.sp);
        self.sp = self.sp.wrapping_add(1);
        u16::from_le_bytes([lo, hi])
    }

    fn read_r8(&self, bus: &Bus, index: u8) -> u8 {
        match index & 0x07 {
            0 => self.regs.b,
            1 => self.regs.c,
            2 => self.regs.d,
            3 => self.regs.e,
            4 => self.regs.h,
            5 => self.regs.l,
            6 => bus.read_byte(self.regs.hl()),
            7 => self.regs.a,
            _ => unreachable!(),
        }
    }

    fn write_r8(&mut self, bus: &mut Bus, index: u8, value: u8) {
        match index & 0x07 {
            0 => self.regs.b = value,
            1 => self.regs.c = value,
            2 => self.regs.d = value,
            3 => self.regs.e = value,
            4 => self.regs.h = value,
            5 => self.regs.l = value,
            6 => {
                let address = self.regs.hl();
                bus.write_byte(address, value);
            }
            7 => self.regs.a = value,
            _ => unreachable!(),
        }
    }

    fn inc8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.regs.set_z(result == 0);
        self.regs.set_n(false);
        self.regs.set_h((value & 0x0F) + 1 > 0x0F);
        result
    }

    fn dec8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.regs.set_z(result == 0);
        self.regs.set_n(true);
        self.regs.set_h((value & 0x0F) == 0);
        result
    }

    fn add_a(&mut self, value: u8, with_carry: bool) {
        let carry_in = u8::from(with_carry && self.regs.flag_c());
        let a = self.regs.a;
        let result = a.wrapping_add(value).wrapping_add(carry_in);
        self.regs.set_z(result == 0);
        self.regs.set_n(false);
        self.regs
            .set_h((a & 0x0F) + (value & 0x0F) + carry_in > 0x0F);
        self.regs
            .set_c((a as u16) + (value as u16) + (carry_in as u16) > 0xFF);
        self.regs.a = result;
    }

    fn sub_a(&mut self, value: u8, with_carry: bool) {
        let carry_in = u8::from(with_carry && self.regs.flag_c());
        let a = self.regs.a;
        let result = a.wrapping_sub(value).wrapping_sub(carry_in);
        self.regs.set_z(result == 0);
        self.regs.set_n(true);
        self.regs.set_h((a & 0x0F) < ((value & 0x0F) + carry_in));
        self.regs
            .set_c((a as u16) < (value as u16) + (carry_in as u16));
        self.regs.a = result;
    }

    fn and_a(&mut self, value: u8) {
        self.regs.a &= value;
        self.regs.set_z(self.regs.a == 0);
        self.regs.set_n(false);
        self.regs.set_h(true);
        self.regs.set_c(false);
    }

    fn xor_a(&mut self, value: u8) {
        self.regs.a ^= value;
        self.regs.set_z(self.regs.a == 0);
        self.regs.set_n(false);
        self.regs.set_h(false);
        self.regs.set_c(false);
    }

    fn or_a(&mut self, value: u8) {
        self.regs.a |= value;
        self.regs.set_z(self.regs.a == 0);
        self.regs.set_n(false);
        self.regs.set_h(false);
        self.regs.set_c(false);
    }

    fn cp_a(&mut self, value: u8) {
        let a = self.regs.a;
        let result = a.wrapping_sub(value);
        self.regs.set_z(result == 0);
        self.regs.set_n(true);
        self.regs.set_h((a & 0x0F) < (value & 0x0F));
        self.regs.set_c(a < value);
    }

    fn add_hl(&mut self, value: u16) {
        let hl = self.regs.hl();
        let result = hl.wrapping_add(value);
        self.regs.set_n(false);
        self.regs.set_h((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.regs
            .set_c((hl as u32).wrapping_add(value as u32) > 0xFFFF);
        self.regs.set_hl(result);
    }

    fn daa(&mut self) {
        let mut correction = 0u8;
        let mut carry = self.regs.flag_c();
        let mut a = self.regs.a;

        if !self.regs.flag_n() {
            if self.regs.flag_h() || (a & 0x0F) > 0x09 {
                correction |= 0x06;
            }
            if carry || a > 0x99 {
                correction |= 0x60;
                carry = true;
            }
            a = a.wrapping_add(correction);
        } else {
            if self.regs.flag_h() {
                correction |= 0x06;
            }
            if carry {
                correction |= 0x60;
            }
            a = a.wrapping_sub(correction);
        }

        self.regs.a = a;
        self.regs.set_z(a == 0);
        self.regs.set_h(false);
        self.regs.set_c(carry);
    }

    fn rlc(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | u8::from(carry);
        self.set_rotate_flags(result, carry, set_zero);
        result
    }

    fn rrc(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | if carry { 0x80 } else { 0 };
        self.set_rotate_flags(result, carry, set_zero);
        result
    }

    fn rl(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry_in = u8::from(self.regs.flag_c());
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | carry_in;
        self.set_rotate_flags(result, carry, set_zero);
        result
    }

    fn rr(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry_in = if self.regs.flag_c() { 0x80 } else { 0 };
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | carry_in;
        self.set_rotate_flags(result, carry, set_zero);
        result
    }

    fn sla(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = value << 1;
        self.set_rotate_flags(result, carry, true);
        result
    }

    fn sra(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | (value & 0x80);
        self.set_rotate_flags(result, carry, true);
        result
    }

    fn srl(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = value >> 1;
        self.set_rotate_flags(result, carry, true);
        result
    }

    fn swap(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(4);
        self.regs.set_z(result == 0);
        self.regs.set_n(false);
        self.regs.set_h(false);
        self.regs.set_c(false);
        result
    }

    fn set_rotate_flags(&mut self, result: u8, carry: bool, set_zero: bool) {
        self.regs.set_z(set_zero && result == 0);
        self.regs.set_n(false);
        self.regs.set_h(false);
        self.regs.set_c(carry);
    }

    fn add_signed_u16(base: u16, offset: i8) -> u16 {
        base.wrapping_add(offset as i16 as u16)
    }

    fn add_sp_offset(sp: u16, offset: u8) -> (u16, bool, bool) {
        let signed = offset as i8 as i16 as u16;
        let result = sp.wrapping_add(signed);
        let half_carry = ((sp ^ signed ^ result) & 0x0010) != 0;
        let carry = ((sp ^ signed ^ result) & 0x0100) != 0;
        (result, half_carry, carry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_steps(gb: &mut GameBoy, steps: usize) {
        for _ in 0..steps {
            gb.step().expect("instruction should execute");
        }
    }

    #[test]
    fn decodes_all_base_and_cb_opcodes() {
        let illegal = [
            0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
        ];

        for opcode in 0u8..=0xFF {
            let mut gb = GameBoy::with_program(0x0000, &[opcode, 0x00, 0x00, 0x00]);
            gb.bus.write_byte(IE_ADDR, 0);
            gb.bus.write_byte(IF_ADDR, 0);
            gb.cpu.pc = 0;

            let result = gb.step();
            if illegal.contains(&opcode) {
                assert!(
                    matches!(result, Err(EmuError::IllegalOpcode(op)) if op == opcode),
                    "opcode 0x{opcode:02X} should be illegal"
                );
            } else {
                assert!(result.is_ok(), "opcode 0x{opcode:02X} should decode");
            }
        }

        for cb_opcode in 0u8..=0xFF {
            let mut gb = GameBoy::with_program(0x0000, &[0xCB, cb_opcode, 0x00]);
            gb.cpu.pc = 0;
            let result = gb.step();
            assert!(
                result.is_ok(),
                "CB opcode 0x{cb_opcode:02X} should decode and execute"
            );
        }
    }

    #[test]
    fn executes_arithmetic_and_cb_flag_logic() {
        let program = [
            0x3E, 0x15, // LD A, 0x15
            0xC6, 0x27, // ADD A, 0x27
            0x27, // DAA -> 0x42
            0xCB, 0x37, // SWAP A -> 0x24
            0xCB, 0x47, // BIT 0, A -> Z set
            0xCB, 0xC7, // SET 0, A -> A = 0x25
        ];

        let mut gb = GameBoy::with_program(0x0000, &program);
        run_steps(&mut gb, 6);

        assert_eq!(gb.cpu.regs.a, 0x25);
        assert!(!gb.cpu.regs.flag_n());
        assert!(gb.cpu.regs.flag_h());
        assert!(gb.cpu.regs.flag_z());
    }

    #[test]
    fn daa_handles_add_sub_and_carry_edge_cases() {
        struct Case {
            a: u8,
            flags: u8,
            expected_a: u8,
            expected_z: bool,
            expected_n: bool,
            expected_h: bool,
            expected_c: bool,
        }

        let cases = [
            Case {
                a: 0x42,
                flags: 0,
                expected_a: 0x42,
                expected_z: false,
                expected_n: false,
                expected_h: false,
                expected_c: false,
            },
            Case {
                a: 0x3C,
                flags: FLAG_H,
                expected_a: 0x42,
                expected_z: false,
                expected_n: false,
                expected_h: false,
                expected_c: false,
            },
            Case {
                a: 0xA0,
                flags: 0,
                expected_a: 0x00,
                expected_z: true,
                expected_n: false,
                expected_h: false,
                expected_c: true,
            },
            Case {
                a: 0x9A,
                flags: FLAG_H | FLAG_C,
                expected_a: 0x00,
                expected_z: true,
                expected_n: false,
                expected_h: false,
                expected_c: true,
            },
            Case {
                a: 0x13,
                flags: FLAG_N | FLAG_H,
                expected_a: 0x0D,
                expected_z: false,
                expected_n: true,
                expected_h: false,
                expected_c: false,
            },
            Case {
                a: 0x40,
                flags: FLAG_N | FLAG_C,
                expected_a: 0xE0,
                expected_z: false,
                expected_n: true,
                expected_h: false,
                expected_c: true,
            },
            Case {
                a: 0x73,
                flags: FLAG_N | FLAG_H | FLAG_C,
                expected_a: 0x0D,
                expected_z: false,
                expected_n: true,
                expected_h: false,
                expected_c: true,
            },
        ];

        for case in cases {
            let mut gb = GameBoy::with_program(0x0000, &[0x27]); // DAA
            gb.cpu.regs.a = case.a;
            gb.cpu.regs.f = case.flags;

            gb.step().expect("DAA should execute");

            assert_eq!(
                gb.cpu.regs.a, case.expected_a,
                "DAA A result mismatch for input A=0x{:02X}, F=0x{:02X}",
                case.a, case.flags
            );
            assert_eq!(gb.cpu.regs.flag_z(), case.expected_z);
            assert_eq!(gb.cpu.regs.flag_n(), case.expected_n);
            assert_eq!(gb.cpu.regs.flag_h(), case.expected_h);
            assert_eq!(gb.cpu.regs.flag_c(), case.expected_c);
        }
    }

    #[test]
    fn services_interrupts_with_priority_and_reti() {
        let mut gb = GameBoy::with_program(0x0000, &[0x00, 0x00, 0x00]);
        gb.bus.load_bytes(0x0040, &[0xD9]); // RETI at VBlank vector
        gb.cpu.ime = true;
        gb.bus
            .write_byte(IE_ADDR, INTERRUPT_VBLANK | INTERRUPT_TIMER);
        gb.bus
            .write_byte(IF_ADDR, INTERRUPT_VBLANK | INTERRUPT_TIMER);

        let cycles = gb.step().expect("interrupt should be serviced");
        assert_eq!(cycles, 20);
        assert_eq!(gb.cpu.pc, 0x0040);
        assert_eq!(gb.cpu.sp, 0xFFFC);
        assert_eq!(gb.bus.read_word(0xFFFC), 0x0000);
        assert!(!gb.cpu.ime);

        let cycles = gb.step().expect("reti should run");
        assert_eq!(cycles, 16);
        assert_eq!(gb.cpu.pc, 0x0000);
        assert!(gb.cpu.ime);
    }

    #[test]
    fn interrupt_dispatch_reacts_to_ie_writes_during_stack_push() {
        // High-byte push to IE can cancel dispatch if it disables the pending interrupt.
        let mut cancel = GameBoy::with_program(0x0200, &[0x00]);
        cancel.cpu.pc = 0x0200;
        cancel.cpu.sp = 0x0000;
        cancel.cpu.ime = true;
        cancel.bus.write_byte(IE_ADDR, INTERRUPT_TIMER);
        cancel.bus.write_byte(IF_ADDR, INTERRUPT_TIMER);
        let cycles = cancel.step().expect("interrupt dispatch should run");
        assert_eq!(cycles, 20);
        assert_eq!(cancel.cpu.pc, 0x0000);
        assert_eq!(cancel.cpu.sp, 0xFFFE);
        assert_eq!(cancel.bus.read_byte(IE_ADDR) & 0x1F, INTERRUPT_LCD);
        assert_eq!(
            cancel.bus.read_byte(IF_ADDR) & INTERRUPT_TIMER,
            INTERRUPT_TIMER
        );
        assert!(!cancel.cpu.ime);

        // Low-byte push to IE must not cancel an already selected dispatch.
        let mut no_cancel = GameBoy::with_program(0x3535, &[0x00]);
        no_cancel.cpu.pc = 0x3535;
        no_cancel.cpu.sp = 0x0001;
        no_cancel.cpu.ime = true;
        no_cancel.bus.write_byte(IE_ADDR, INTERRUPT_SERIAL);
        no_cancel.bus.write_byte(IF_ADDR, INTERRUPT_SERIAL);
        let cycles = no_cancel.step().expect("interrupt dispatch should run");
        assert_eq!(cycles, 20);
        assert_eq!(no_cancel.cpu.pc, 0x0058);
        assert_eq!(no_cancel.bus.read_byte(IF_ADDR) & INTERRUPT_SERIAL, 0);

        // High-byte push can retarget to a different pending interrupt.
        let mut retarget = GameBoy::with_program(0x0200, &[0x00]);
        retarget.cpu.pc = 0x0200;
        retarget.cpu.sp = 0x0000;
        retarget.cpu.ime = true;
        retarget
            .bus
            .write_byte(IE_ADDR, INTERRUPT_VBLANK | INTERRUPT_LCD);
        retarget
            .bus
            .write_byte(IF_ADDR, INTERRUPT_VBLANK | INTERRUPT_LCD);
        let cycles = retarget.step().expect("interrupt dispatch should run");
        assert_eq!(cycles, 20);
        assert_eq!(retarget.cpu.pc, 0x0048);
        assert_eq!(
            retarget.bus.read_byte(IF_ADDR) & INTERRUPT_VBLANK,
            INTERRUPT_VBLANK
        );
        assert_eq!(retarget.bus.read_byte(IF_ADDR) & INTERRUPT_LCD, 0);
    }

    #[test]
    fn enables_ime_after_instruction_following_ei() {
        let mut gb = GameBoy::with_program(0x0000, &[0xFB, 0x00, 0x00]); // EI, NOP, NOP
        gb.bus.load_bytes(0x0050, &[0xD9]); // RETI for timer interrupt
        gb.bus.write_byte(IE_ADDR, INTERRUPT_TIMER);
        gb.bus.write_byte(IF_ADDR, INTERRUPT_TIMER);

        gb.step().expect("EI");
        assert!(!gb.cpu.ime);

        gb.step().expect("NOP after EI");
        assert!(gb.cpu.ime);
        assert_eq!(gb.cpu.pc, 0x0002);

        gb.step()
            .expect("interrupt should preempt instruction fetch");
        assert_eq!(gb.cpu.pc, 0x0050);
    }

    #[test]
    fn timer_ticks_and_overflow_reload_request_interrupt() {
        let mut bus = Bus::default();
        bus.write_byte(TAC_ADDR, 0b101); // enable, clock=16 cycles
        bus.write_byte(TIMA_ADDR, 0x00);

        bus.tick(16);
        assert_eq!(bus.read_byte(TIMA_ADDR), 0x01);

        bus.write_byte(TIMA_ADDR, 0xFF);
        bus.write_byte(TMA_ADDR, 0xAC);
        bus.write_byte(IF_ADDR, 0x00);

        bus.tick(16);
        assert_eq!(bus.read_byte(TIMA_ADDR), 0x00);
        assert_eq!(bus.read_byte(IF_ADDR) & INTERRUPT_TIMER, 0);

        bus.tick(4);
        assert_eq!(bus.read_byte(TIMA_ADDR), 0xAC);
        assert_ne!(bus.read_byte(IF_ADDR) & INTERRUPT_TIMER, 0);
    }

    #[test]
    fn serial_port_capture_records_transfer_bytes() {
        let mut bus = Bus::default();
        bus.write_byte(SB_ADDR, b'O');
        bus.write_byte(SC_ADDR, 0x81);
        bus.write_byte(SB_ADDR, b'K');
        bus.write_byte(SC_ADDR, 0x80);
        bus.write_byte(SC_ADDR, 0x81);

        assert_eq!(bus.serial_output(), b"OK");
        assert_eq!(bus.read_byte(SC_ADDR) & 0x80, 0);

        let output = bus.take_serial_output();
        assert_eq!(output.as_slice(), b"OK");
        assert!(bus.serial_output().is_empty());
    }

    #[test]
    fn halt_idles_and_halt_bug_reuses_pc() {
        let mut halted = GameBoy::with_program(0x0000, &[0x76, 0x00]); // HALT, NOP
        halted.step().expect("HALT");
        assert!(halted.cpu.halted);
        let pc = halted.cpu.pc;
        let cycles = halted.step().expect("halted idle");
        assert_eq!(cycles, 4);
        assert_eq!(halted.cpu.pc, pc);

        let mut halt_bug = GameBoy::with_program(0x0000, &[0x76, 0x04, 0x00]); // HALT, INC B, NOP
        halt_bug.cpu.ime = false;
        halt_bug.bus.write_byte(IE_ADDR, INTERRUPT_VBLANK);
        halt_bug.bus.write_byte(IF_ADDR, INTERRUPT_VBLANK);

        halt_bug
            .step()
            .expect("HALT with pending interrupt triggers bug");
        assert!(!halt_bug.cpu.halted);
        assert_eq!(halt_bug.cpu.pc, 0x0001);

        halt_bug.step().expect("INC B with halted bug fetch");
        assert_eq!(halt_bug.cpu.regs.b, 1);
        assert_eq!(halt_bug.cpu.pc, 0x0001);

        halt_bug.step().expect("INC B fetched again");
        assert_eq!(halt_bug.cpu.regs.b, 2);
        assert_eq!(halt_bug.cpu.pc, 0x0002);
    }

    #[test]
    fn stop_state_waits_for_interrupt_and_then_resumes() {
        let mut gb = GameBoy::with_program(0x0000, &[0x10, 0x00, 0x00]); // STOP 00, NOP
        gb.step().expect("STOP");
        assert!(gb.cpu.stopped);
        assert_eq!(gb.cpu.pc, 0x0002);

        gb.step().expect("stopped idle");
        assert_eq!(gb.cpu.pc, 0x0002);

        gb.bus.write_byte(IE_ADDR, INTERRUPT_JOYPAD);
        gb.bus.write_byte(IF_ADDR, INTERRUPT_JOYPAD);
        gb.step().expect("resume from stop");
        assert!(!gb.cpu.stopped);
        assert_eq!(gb.cpu.pc, 0x0003);
    }
}
