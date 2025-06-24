use std::error::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, mpsc};
use std::{thread, time};

use crate::interrupts::InterruptFlag;

use super::bus::{HardwareRegister, MemoryBus};
use super::cart::Cartridge;
use super::cpu::*;
use super::dma::DMA;
use super::gui::{GUI, GuiAction};
use super::interrupts::InterruptLine;
use super::ppu::PPU;
use super::timer::Timer;

/// The main emulator state.
///
/// The emulator is composed of the following components:
/// - Cartridge
/// - CPU
/// - Address bus
/// - PPU (Pixel Processing Unit)
/// - Timer
///
// #[derive(Debug)]
pub struct Emulator {
    ticks: u64,
    bus: MemoryBus,
    interrupts: InterruptLine,
    dma: DMA,
    ppu: PPU,
    timer: Timer,
    debug_msg: String,
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
            self.timer.tick(&mut self.interrupts);
            self.ppu.tick(&mut self.interrupts);
        }

        self.dma.tick_cycle(&self.bus, &mut self.ppu);
    }

    fn read_cycle(&mut self, address: u16) -> u8 {
        let value = self.peek(address);
        self.tick_cycle();
        value
    }

    fn write_cycle(&mut self, address: u16, value: u8) {
        // Write everything to bus just in case
        self.bus.write(address, value);

        match address {
            0x8000..=0x9FFF => self.ppu.vram_write(address, value),
            0xFE00..=0xFE9F => {
                if self.dma.is_active() {
                    return;
                }
                self.ppu.oam_write(address, value);
            }
            0xFF00..=0xFF7F | 0xFFFF => {
                let register = HardwareRegister::from_u16(address);
                match register {
                    Some(HardwareRegister::SB) => {
                        self.bus.write(address, value);
                        let serial_transfer_requested =
                            self.bus.read_register(HardwareRegister::SC) == 0x81;

                        if serial_transfer_requested {
                            self.debug_msg.push(value as char);
                            self.bus.write_register(HardwareRegister::SC, 0);
                        }
                    }
                    Some(HardwareRegister::SC) => self.bus.write(address, value),
                    Some(HardwareRegister::DIV)
                    | Some(HardwareRegister::TIMA)
                    | Some(HardwareRegister::TMA)
                    | Some(HardwareRegister::TAC) => {
                        self.timer.write(address, value);
                    }
                    Some(HardwareRegister::IF) => {
                        self.interrupts.interrupt_flag = InterruptFlag::from_bits_truncate(value);
                    }
                    Some(HardwareRegister::LCDC)
                    | Some(HardwareRegister::STAT)
                    | Some(HardwareRegister::SCY)
                    | Some(HardwareRegister::SCX)
                    | Some(HardwareRegister::LY)
                    | Some(HardwareRegister::LYC)
                    | Some(HardwareRegister::BGP)
                    | Some(HardwareRegister::OBP0)
                    | Some(HardwareRegister::OBP1)
                    | Some(HardwareRegister::WY)
                    | Some(HardwareRegister::WX) => {
                        self.ppu.lcd_write(register.unwrap(), value);
                    }
                    // TODO: Should we move DMA to LCD/PPU?
                    Some(HardwareRegister::DMA) => self.dma.start(value),
                    Some(HardwareRegister::IE) => {
                        self.interrupts.interrupt_enable = InterruptFlag::from_bits_truncate(value);
                    }
                    _ => println!("Unimplemented hardware register write ${:04X}.", address),
                };
            }
            _ => (),
        }
        self.tick_cycle();
    }

    fn get_interrupt(&mut self) -> Option<InterruptFlag> {
        // TODO: How the bus should update these values?
        let ier = self.interrupts.interrupt_enable.bits();
        let ifr = self.interrupts.interrupt_flag.bits();

        let bus_ier = self.bus.read_register(HardwareRegister::IE);
        let bus_ifr = self.bus.read_register(HardwareRegister::IF);

        if bus_ier != ier || bus_ifr != ifr {
            //panic!("Interrupt registers are not synchronized.");
        }

        if (ier & ifr) != 0 {
            return Some(InterruptFlag::from_bits_truncate(ier & ifr));
        }

        None
    }

    /// Clear the interrupt flag
    fn ack_interrupt(&mut self, f: &InterruptFlag) {
        let ifr = self.interrupts.interrupt_flag.bits();
        let new_ifr = ifr & !(f.highest_priority().bits());
        self.interrupts.interrupt_flag = InterruptFlag::from_bits_truncate(new_ifr);
        // TODO: How the bus should update these values?
        self.bus.write_register(HardwareRegister::IF, new_ifr);
    }

    fn peek(&mut self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => self.ppu.vram_read(address),
            0xFE00..=0xFE9F => {
                if self.dma.is_active() {
                    return 0xFF;
                }
                self.ppu.oam_read(address)
            }
            0xFF00..=0xFF7F | 0xFFFF => {
                let register = HardwareRegister::from_u16(address);
                match register {
                    Some(HardwareRegister::SB) | Some(HardwareRegister::SC) => {
                        self.bus.read(address)
                    }
                    Some(HardwareRegister::DIV)
                    | Some(HardwareRegister::TIMA)
                    | Some(HardwareRegister::TMA)
                    | Some(HardwareRegister::TAC) => self.timer.read(address),
                    Some(HardwareRegister::IF) => self.interrupts.interrupt_flag.bits(),

                    Some(HardwareRegister::LCDC)
                    | Some(HardwareRegister::STAT)
                    | Some(HardwareRegister::SCY)
                    | Some(HardwareRegister::SCX)
                    | Some(HardwareRegister::LY)
                    | Some(HardwareRegister::LYC)
                    | Some(HardwareRegister::BGP)
                    | Some(HardwareRegister::OBP0)
                    | Some(HardwareRegister::OBP1)
                    | Some(HardwareRegister::WY)
                    | Some(HardwareRegister::WX) => self.ppu.lcd_read(register.unwrap()),
                    Some(HardwareRegister::IE) => self.interrupts.interrupt_enable.bits(),
                    _ => {
                        println!("Unimplemented hardware register read ${:02X}.", address);
                        self.bus.read(address)
                    }
                }
            }
            _ => self.bus.read(address),
        }
    }

    fn ticks(&self) -> u64 {
        self.ticks
    }
}

impl Emulator {
    pub fn delay(ms: u64) {
        let d_ms = time::Duration::from_millis(ms);
        thread::sleep(d_ms);
    }

    pub fn new() -> Self {
        Emulator {
            ticks: 0,
            bus: MemoryBus::new(),
            interrupts: InterruptLine::new(),
            dma: DMA::new(),
            ppu: PPU::new(),
            timer: Timer::new(),
            debug_msg: String::new(),
        }
    }

    pub fn run(rom_file: &str) -> Result<(), Box<dyn Error>> {
        let emu_mutex = Arc::new(Mutex::new(Emulator::new()));
        println!("Reading {rom_file}");
        let rom = Cartridge::load(rom_file)?;
        let mut gui: GUI = GUI::new(true);
        CPU_DEBUG_LOG.set(false).unwrap();

        {
            let mut emu = emu_mutex.lock().unwrap();
            emu.bus.set_rom(Some(rom));
        }

        let mut cpu: CPU = CPU::new(emu_mutex.clone());
        println!("CPU initialized\n{}", cpu);

        let (tx, rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();

        thread::spawn(move || {
            loop {
                if !cpu.step() {
                    println!("CPU stopped.");
                    tx.send(false).unwrap();
                }
            }
        });

        let mut prev_frame: u32 = 0;

        loop {
            let action: GuiAction = gui.handle_events();

            if action == GuiAction::Exit {
                return Ok(());
            }

            {
                let emu = emu_mutex.lock().unwrap();

                if prev_frame != emu.ppu.get_current_frame() {
                    prev_frame = emu.ppu.get_current_frame();
                    gui.update_debug_window(&emu.ppu);
                }

                // For testing
                if !emu.debug_msg.is_empty() && emu.debug_msg.contains("Passed") {
                    panic!("Debug message: {}", emu.debug_msg);
                }
            }

            match rx.try_recv() {
                Ok(running) => {
                    if !running {
                        return Ok(());
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Ok(());
                }
                Err(mpsc::TryRecvError::Empty) => (),
            };

            // Limit frame rate to 60Hz
            Emulator::delay(16);
        }
    }
}
