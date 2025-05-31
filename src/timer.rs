use bitflags::bitflags;

use crate::{bus::HardwareRegister, interrupts::InterruptFlag};

use super::interrupts::InterruptRequest;

bitflags!(
    pub struct TacRegister: u8 {
        const ENABLE = 0b100;
        const CLOCK1 = 0b010;
        const CLOCK0 = 0b001;
    }
);

pub struct Timer {
    pub div: u16, // Internal system counter
    pub tima: u8,
    pub tma: u8,
    pub tac: TacRegister,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0xAC00, // In docs, 0xABCC specified for DMG
            tima: 0,
            tma: 0,
            tac: TacRegister::from_bits_truncate(0),
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        match HardwareRegister::from_u16(address) {
            Some(HardwareRegister::DIV) => (self.div >> 8) as u8,
            Some(HardwareRegister::TIMA) => self.tima,
            Some(HardwareRegister::TMA) => self.tma,
            Some(HardwareRegister::TAC) => self.tac.bits(),
            _ => panic!("Invalid timer register {}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match HardwareRegister::from_u16(address) {
            Some(HardwareRegister::DIV) => self.div = 0,
            Some(HardwareRegister::TIMA) => self.tima = value,
            Some(HardwareRegister::TMA) => self.tma = value,
            Some(HardwareRegister::TAC) => self.tac = TacRegister::from_bits_truncate(value),
            _ => panic!("Invalid timer register {}", address),
        }
    }

    pub fn tick<I: InterruptRequest>(&mut self, ctx: &mut I) {
        let prev_div = self.div;
        self.div = self.div.wrapping_add(1);
        // The DIV register acts as the source clock,
        // specific bits of DIV are used to trigger TIMA updates:
        //     DIV[9] for 4096 Hz.
        //     DIV[3] for 262144 Hz.
        //     DIV[5] for 65536 Hz.
        //     DIV[7] for 16384 Hz.
        if self.tac.contains(TacRegister::ENABLE) {
            let timer_update = match self.tac.bits() & 0b11 {
                0b00 => (prev_div & (1 << 9)) != 0 && (self.div & (1 << 9)) == 0,
                0b01 => (prev_div & (1 << 3)) != 0 && (self.div & (1 << 3)) == 0,
                0b10 => (prev_div & (1 << 5)) != 0 && (self.div & (1 << 5)) == 0,
                0b11 => (prev_div & (1 << 7)) != 0 && (self.div & (1 << 7)) == 0,
                _ => false,
            };

            if timer_update {
                self.tima = self.tima.wrapping_add(1);

                if self.tima == 0xFF {
                    self.tima = self.tma;
                    ctx.request_interrupt(InterruptFlag::TIMER);
                }
            }
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
