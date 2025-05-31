use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::{thread, time};

use super::bus::{HardwareRegister, MemoryBus};
use super::cart::Cartridge;
use super::cpu::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

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
    paused: bool,
    running: bool,
    ticks: u64,
    bus: MemoryBus,
    sdl_context: Option<sdl2::Sdl>,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
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
            sdl_context: None,
            canvas: None,
            debug_msg: String::new(),
        }
    }

    pub fn run(rom_file: &str) -> Result<(), Box<dyn Error>> {
        let emu = Rc::new(RefCell::new(Emulator::new()));
        println!("Reading {rom_file}");
        let rom = Cartridge::load(rom_file)?;
        emu.borrow_mut().bus.set_rom(Some(rom));
        let mut cpu = CPU::new(emu.clone());

        println!("CPU initialized\n{}", cpu);

        emu.borrow_mut().ui_init();
        emu.borrow_mut().running = true;

        while emu.borrow().running {
            emu.borrow_mut().ui_handle_events();

            if emu.borrow().paused {
                Emulator::delay(100);
                continue;
            }

            if !cpu.step() {
                println!("CPU stopped.");
                return Ok(());
            }

            emu.borrow_mut().debug_update();

            if !emu.borrow().debug_msg.is_empty() {
                println!("Debug message: {}", emu.borrow().debug_msg);
            }

            // Limit frame rate to 60Hz
            Emulator::delay(16);
            emu.borrow_mut().ticks += 1;
        }

        Ok(())
    }

    fn ui_init(&mut self) {
        const SCREEN_WIDTH: u32 = 20;
        const SCREEN_HEIGHT: u32 = 18;
        const SCALE: u32 = 5;

        if self.sdl_context.is_none() {
            self.sdl_context = Some(sdl2::init().unwrap());
        }

        let video_subsystem = self.sdl_context.as_ref().unwrap().video().unwrap();
        let window = video_subsystem
            .window(
                "GameBoy Emulator",
                SCREEN_WIDTH * 8 * SCALE,
                SCREEN_HEIGHT * 8 * SCALE,
            )
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        self.canvas = Some(canvas);
    }

    fn ui_handle_events(&mut self) {
        let mut event_pump = self.sdl_context.as_ref().unwrap().event_pump().unwrap();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => self.running = false,
                _ => {}
            }
        }
    }

    fn debug_update(&mut self) {
        let serial_transfer_requested = self.bus.read_register(HardwareRegister::SC) == 0x81;

        if serial_transfer_requested {
            let c = self.bus.read_register(HardwareRegister::SB);
            self.debug_msg.push(c as char);
            self.bus.write_register(HardwareRegister::SC, 0);
        }
    }
}
