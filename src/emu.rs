use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::{thread, time};

use super::bus::MemoryBus;
use super::cart::Cartridge;
use super::cpu::*;

/// The main emulator state.
///
/// The emulator is composed of the following components:
/// - Cartridge
/// - CPU
/// - Address bus
/// - PPU (Pixel Processing Unit)
/// - Timer
///
#[derive(Debug)]
pub struct Emulator {
    paused: bool,
    running: bool,
    ticks: u64,
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuContext for Emulator {
    fn tick_cycle(&mut self) {
        todo!();
    }
}

impl Emulator {
    pub fn delay(ms: u64) {
        let d_ms = time::Duration::from_millis(ms);
        thread::sleep(d_ms);
    }

    pub fn new() -> Self {
        Emulator {
            paused: false,
            running: false,
            ticks: 0,
        }
    }

    pub fn run(rom_file: &str) -> Result<(), Box<dyn Error>> {
        let emu = Rc::new(RefCell::new(Emulator::new()));
        let _rom = Cartridge::load(rom_file)?;
        let mut bus = MemoryBus::new();
        let cpu = CPU::new(&mut bus, emu.clone());

        emu.borrow_mut().running = true;

        while emu.borrow().running {
            if emu.borrow().paused {
                Emulator::delay(10);
                continue;
            }

            if !cpu.step() {
                println!("CPU stopped.");
                return Ok(());
            }

            emu.borrow_mut().ticks += 1;
        }

        Ok(())
    }
}
