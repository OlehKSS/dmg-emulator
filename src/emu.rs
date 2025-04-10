use std::error::Error;
use std::{thread, time};

use super::cart::Cartridge;
use super::cpu::CPU;

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

    pub fn run(&mut self, rom_file: &str) -> Result<(), Box<dyn Error>> {
        let _rom = Cartridge::load(rom_file)?;
        let cpu = CPU::new();

        self.running = true;

        while self.running {
            if self.paused {
                Emulator::delay(10);
                continue;
            }

            if !cpu.step() {
                println!("CPU stopped.");
                return Ok(());
            }

            self.ticks += 1;
        }

        Ok(())
    }
}
