use bitflags::bitflags;
use std::thread;
use std::time::{Duration, Instant};

use crate::bus::HardwareRegister;
use crate::interrupts::InterruptFlag;
use crate::lcd::LcdStatus;

use super::interrupts::InterruptRequest;
use super::lcd::{LCD, LcdMode};

bitflags!(
/// Priority: 0 = No, 1 = BG and Window color indices 1–3 are drawn over this OBJ
/// Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
/// X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
/// DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
/// Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
/// CGB palette [CGB Mode Only]: Which of OBP0–7 to use
    pub struct SpriteFlags: u8 {
        const PRIORITY = 0b1000_0000;
        const Y_FLIP = 0b0100_0000;
        const X_FLIP = 0b0010_0000;
        const DMG_PALETTE = 0b0001_0000;
        const BANK = 0b0000_1000;
        const CGB_PALETTE2 = 0b0000_0100;
        const CGB_PALETTE1 = 0b0000_0010;
        const CGB_PALETTE0 = 0b0000_0001;
    }
);

/// PPU (Pixel Processing Unit)
///
/// OAM (Object Attribute Memory) RAM stores sprite information.
/// It holds 40 sprites in total, 4 bytes each.
///
/// DMG has 8KB (0x2000) of VRAM (Video RAM) located at 0x8000–0x9FFF.
///
/// Breakdown of VRAM Usage:
/// 1. Tile Data (0x8000–0x97FF):
///     * Stores graphical data for tiles used in backgrounds and sprites.
///     * Each tile is 8x8 pixels, with 2 bits per pixel for color, 16 bytes in total.
/// 2. Tile Maps (0x9800–0x9BFF and 0x9C00–0x9FFF):
///     * Stores the arrangement of tiles for the background.
///     * Two separate tile maps are available, allowing for different layouts.
const OAM_SIZE: usize = 0xA0;
const VRAM_SIZE: usize = 0x2000;
const LINES_PER_FRAME: u32 = 154;
const TICKS_PER_LINE: u32 = 456;
const YRES: usize = 144;
const XRES: usize = 160;
// Target frame rate is 60 Hz
const TARGET_FRAME_TIME: Duration = Duration::from_millis(16);

pub struct PPU {
    oam_ram: [u8; OAM_SIZE],
    vram: [u8; VRAM_SIZE], // 8KB
    lcd: LCD,
    timer: Instant,
    start_time: Duration,
    prev_frame_time: Duration,
    frame_count: u32,
    current_frame: u32,
    line_ticks: u32,
    video_buffer: [u32; YRES * XRES],
}

impl PPU {
    pub fn new() -> Self {
        let mut lcd = LCD::new();
        lcd.set_mode(LcdMode::OAM);

        PPU {
            oam_ram: [0; OAM_SIZE],
            vram: [0; VRAM_SIZE],
            lcd,
            timer: Instant::now(),
            start_time: Duration::from_millis(0),
            prev_frame_time: Duration::from_millis(0),
            frame_count: 0,
            current_frame: 0,
            line_ticks: 0,
            video_buffer: [0; YRES * XRES],
        }
    }

    pub fn get_current_frame(&self) -> u32 {
        self.current_frame
    }

    pub fn oam_read(&self, address: u16) -> u8 {
        // Both ranges are valid, one is for DMA
        let oam_address = if address >= 0xFE00 {
            (address - 0xFE00) as usize
        } else {
            address as usize
        };
        self.oam_ram[oam_address]
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        let oam_address = if address >= 0xFE00 {
            (address - 0xFE00) as usize
        } else {
            address as usize
        };
        self.oam_ram[oam_address] = value;
    }

    pub fn vram_read(&self, address: u16) -> u8 {
        let vram_address = (address - 0x8000) as usize;
        self.vram[vram_address]
    }

    pub fn vram_write(&mut self, address: u16, value: u8) {
        let vram_address = (address - 0x8000) as usize;
        self.vram[vram_address] = value;
    }

    pub fn lcd_read(&self, register: HardwareRegister) -> u8 {
        self.lcd.read(register)
    }

    pub fn lcd_write(&mut self, register: HardwareRegister, value: u8) {
        self.lcd.write(register, value);
    }

    pub fn tick<I: InterruptRequest>(&mut self, ctx: &mut I) {
        self.line_ticks += 1;
        let lcd_mode = self.lcd.get_mode();

        match lcd_mode {
            LcdMode::OAM => {
                if self.line_ticks >= 80 {
                    self.lcd.set_mode(LcdMode::XFER);
                }
            }
            LcdMode::XFER => {
                if self.line_ticks >= 80 + 172 {
                    self.lcd.set_mode(LcdMode::HBLANK);
                }
            }
            LcdMode::VBLANK => {
                if self.line_ticks >= TICKS_PER_LINE {
                    self.lcd.increment_ly(ctx);

                    if (self.lcd.ly as u32) >= LINES_PER_FRAME {
                        self.lcd.set_mode(LcdMode::OAM);
                        self.lcd.ly = 0;
                    }

                    self.line_ticks = 0;
                }
            }
            LcdMode::HBLANK => {
                if self.line_ticks >= TICKS_PER_LINE {
                    self.lcd.increment_ly(ctx);

                    if (self.lcd.ly as usize) >= YRES {
                        self.lcd.set_mode(LcdMode::VBLANK);

                        ctx.request_interrupt(InterruptFlag::VBLANK);

                        if self.lcd.status_contains(LcdStatus::VBLANK_INT_SELECT) {
                            ctx.request_interrupt(InterruptFlag::LCD);
                        }

                        self.current_frame += 1;

                        let end = self.timer.elapsed();
                        let frame_time = end - self.prev_frame_time;

                        if frame_time < TARGET_FRAME_TIME {
                            thread::sleep(TARGET_FRAME_TIME - frame_time);
                        }

                        // TODO: Can we make it an overlay on our window by moving to emu.rs?
                        if (end - self.start_time).as_millis() > 1000 {
                            println!("FPS: {}", self.frame_count);
                            self.start_time = end;
                            self.frame_count = 0;
                        }

                        self.frame_count += 1;
                        self.prev_frame_time = self.timer.elapsed();
                    } else {
                        self.lcd.set_mode(LcdMode::OAM);
                    }

                    self.line_ticks = 0;
                }
            }
        }
    }
}

impl Default for PPU {
    fn default() -> Self {
        PPU::new()
    }
}

struct Sprite {
    y_pos: u8,
    x_pos: u8,
    tile_index: u8,
    flags: SpriteFlags,
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y_pos: 0,
            x_pos: 0,
            tile_index: 0,
            flags: SpriteFlags::empty(),
        }
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite::new()
    }
}
