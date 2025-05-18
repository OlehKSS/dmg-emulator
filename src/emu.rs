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
    bus: MemoryBus,
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuContext for Emulator {
    fn tick_cycle(&mut self) {
        // 1 Memory cycle is 4 CPU cycle
        for _ in 0..4 {
            self.ticks += 1;
            // TODO: add timer
        }
    }

    fn read_cycle(&mut self, address: u16) -> u8 {
        self.tick_cycle();
        self.bus.read(address)
    }

    fn write_cycle(&mut self, address: u16, value: u8) {
        self.tick_cycle();
        self.bus.write(address, value);
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
            bus: MemoryBus::new(),
        }
    }

    pub fn run(rom_file: &str) -> Result<(), Box<dyn Error>> {
        let emu = Rc::new(RefCell::new(Emulator::new()));
        let rom = Cartridge::load(rom_file)?;
        emu.borrow_mut().bus.set_rom(Some(rom));
        let mut cpu = CPU::new(emu.clone());

        println!("CPU initialized\n{}", cpu);

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
