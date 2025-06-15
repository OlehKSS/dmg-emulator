use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::{thread, time};

use crate::interrupts::InterruptFlag;

use super::bus::{HardwareRegister, MemoryBus};
use super::cart::Cartridge;
use super::cpu::*;
use super::dma::DMA;
use super::interrupts::InterruptLine;
use super::ppu::PPU;
use super::timer::Timer;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

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
    interrupts: InterruptLine,
    dma: DMA,
    ppu: PPU,
    timer: Timer,
    sdl_context: Option<sdl2::Sdl>,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
    debug_canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
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
        }

        self.dma.tick_cycle(&self.bus, &mut self.ppu);
    }

    fn read_cycle(&mut self, address: u16) -> u8 {
        self.tick_cycle();
        self.peek(address)
    }

    fn write_cycle(&mut self, address: u16, value: u8) {
        self.tick_cycle();
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
                match HardwareRegister::from_u16(address) {
                    Some(HardwareRegister::SB) | Some(HardwareRegister::SC) => {
                        self.bus.write(address, value)
                    }
                    Some(HardwareRegister::DIV)
                    | Some(HardwareRegister::TIMA)
                    | Some(HardwareRegister::TMA)
                    | Some(HardwareRegister::TAC) => {
                        self.timer.write(address, value);
                    }
                    Some(HardwareRegister::IF) => {
                        self.interrupts.interrupt_flag = InterruptFlag::from_bits_truncate(value);
                    }
                    Some(HardwareRegister::DMA) => self.dma.start(value),
                    Some(HardwareRegister::IE) => {
                        self.interrupts.interrupt_enable = InterruptFlag::from_bits_truncate(value);
                    }
                    _ => println!("Unimplemented hardware register write ${:04X}.", address),
                };
            }
            _ => (),
        }
    }

    fn get_interrupt(&mut self) -> Option<InterruptFlag> {
        // TODO: How the bus should update these values?
        let ier = self.interrupts.interrupt_enable.bits();
        let ifr = self.interrupts.interrupt_flag.bits();

        let bus_ier = self.bus.read_register(HardwareRegister::IE);
        let bus_ifr = self.bus.read_register(HardwareRegister::IF);

        if bus_ier != ier || bus_ifr != ifr {
            panic!("Interrupt registers are not synchronized.");
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
            0xFF00..=0xFF7F | 0xFFFF => match HardwareRegister::from_u16(address) {
                Some(HardwareRegister::SB) | Some(HardwareRegister::SC) => self.bus.read(address),
                Some(HardwareRegister::DIV)
                | Some(HardwareRegister::TIMA)
                | Some(HardwareRegister::TMA)
                | Some(HardwareRegister::TAC) => self.timer.read(address),
                Some(HardwareRegister::IF) => self.interrupts.interrupt_flag.bits(),
                Some(HardwareRegister::LY) => self.bus.read(address),
                Some(HardwareRegister::IE) => self.interrupts.interrupt_enable.bits(),
                _ => {
                    println!("Unimplemented hardware register read ${:02X}.", address);
                    self.bus.read(address)
                }
            },
            _ => self.bus.read(address),
        }
    }

    fn ticks(&self) -> u64 {
        self.ticks
    }
}

impl Emulator {
    const SCREEN_WIDTH: u32 = 20;
    const SCREEN_HEIGHT: u32 = 18;
    const DEBUG_SCREEN_WIDTH: u32 = 16;
    const DEBUG_SCREEN_HEIGHT: u32 = 24;
    const SCALE: u32 = 5;

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
            interrupts: InterruptLine::new(),
            dma: DMA::new(),
            ppu: PPU::new(),
            timer: Timer::new(),
            sdl_context: None,
            canvas: None,
            debug_canvas: None,
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

            emu.borrow_mut().update_debug_window();

            // Limit frame rate to 60Hz
            Emulator::delay(16);
        }

        Ok(())
    }

    fn ui_init(&mut self) {
        if self.sdl_context.is_none() {
            self.sdl_context = Some(sdl2::init().unwrap());
        }

        let video_subsystem = self.sdl_context.as_ref().unwrap().video().unwrap();
        let window = video_subsystem
            .window(
                "GameBoy Emulator",
                Self::SCREEN_WIDTH * 8 * Self::SCALE,
                Self::SCREEN_HEIGHT * 8 * Self::SCALE,
            )
            .position_centered()
            .build()
            .unwrap();

        let (posx, posy) = window.position();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        self.canvas = Some(canvas);

        let debug_window = video_subsystem
            .window(
                "Debug Info",
                Self::DEBUG_SCREEN_WIDTH * 8 * Self::SCALE + Self::DEBUG_SCREEN_WIDTH * Self::SCALE,
                Self::DEBUG_SCREEN_HEIGHT * 8 * Self::SCALE
                    + Self::DEBUG_SCREEN_HEIGHT * Self::SCALE,
            )
            .position(
                posx + (((Self::SCREEN_WIDTH + 1) * 8 * Self::SCALE) as i32),
                posy,
            )
            .build()
            .unwrap();

        let mut debug_canvas = debug_window.into_canvas().build().unwrap();
        debug_canvas.set_draw_color(Color::RGB(100, 0, 0));
        debug_canvas.clear();
        debug_canvas.present();

        self.debug_canvas = Some(debug_canvas);
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

    fn update_debug_window(&mut self) {
        let mut x_draw = 0i32;
        let mut y_draw = 0i32;
        let mut tile_num = 0u16;
        let scale = Self::SCALE as i32;

        for y in 0..Self::DEBUG_SCREEN_HEIGHT {
            for x in 0..Self::DEBUG_SCREEN_WIDTH {
                let x_tile = x_draw + ((x as i32) * scale);
                let y_tile = y_draw + ((y as i32) * scale);
                self.display_tile(tile_num, x_tile, y_tile);
                x_draw += 8 * scale;
                tile_num += 1;
            }
            y_draw += 8 * scale;
            x_draw = 0;
        }

        self.debug_canvas.as_mut().unwrap().present();
    }

    fn display_tile(&mut self, tile_num: u16, x: i32, y: i32) {
        const GREEN_LIGHT: Color = Color::RGB(144, 238, 144);
        const GREEN_MEDIUM: Color = Color::RGB(0, 128, 0);
        const GREEN_DARK: Color = Color::RGB(0, 100, 0);
        const GREEN_FOREST: Color = Color::RGB(34, 139, 34);

        let colors = [GREEN_LIGHT, GREEN_MEDIUM, GREEN_DARK, GREEN_FOREST];

        const START_ADDRESS: u16 = 0x8000;
        let scale = Self::SCALE as i32;

        for tile_byte in (0..16u16).step_by(2) {
            let b1 = self
                .ppu
                .vram_read(START_ADDRESS + tile_num * 16 + tile_byte);
            let b2 = self
                .ppu
                .vram_read(START_ADDRESS + tile_num * 16 + tile_byte + 1);

            for bit in (0..=7u16).rev() {
                let hi = ((b1 & (1 << bit)) != 0) as u8;
                let lo = ((b2 & (1 << bit)) != 0) as u8;
                let color_index = ((hi << 1) | lo) as usize;
                let color = colors[color_index];

                let x_rc = x + (((7 - bit) as i32) * scale);
                let y_rc = y + (tile_byte as i32) / 2 * scale;
                let rc = Rect::new(x_rc, y_rc, Self::SCALE, Self::SCALE);

                self.debug_canvas.as_mut().unwrap().set_draw_color(color);
                self.debug_canvas.as_mut().unwrap().fill_rect(rc).unwrap();
            }
        }
    }
}
