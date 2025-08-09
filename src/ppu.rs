use bitflags::bitflags;
use std::collections::VecDeque;
use std::thread;
use std::time::{Duration, Instant};

use crate::bus::HardwareRegister;
use crate::interrupts::InterruptFlag;
use crate::lcd::{LcdControl, LcdStatus};

use super::interrupts::InterruptRequest;
use super::lcd::{LCD, LcdMode};

bitflags!(
/// Priority: 0 = No, 1 = BG and Window color indices 1–3 are drawn over this OBJ
/// Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
/// X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
/// DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
/// Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
/// CGB palette [CGB Mode Only]: Which of OBP0–7 to use
    #[derive(Clone, Copy)]
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

#[derive(Copy, Clone, Debug, PartialEq)]
enum FetchState {
    Tile,
    DataLow,
    DataHigh,
    Idle,
    Push,
}

type Color = u32;

struct PixelFifo {
    fetch_state: FetchState,
    fifo: VecDeque<Color>,
    line_x: u8,
    pushed_x: u8,
    fetch_x: u8,
    bgw_fetch_data: [u8; 3],
    fetch_entry_data: [u8; 6], // OAM data
    map_y: u8,
    map_x: u8,
    tile_y: u8,
    fifo_x: u8,
}

impl PixelFifo {
    pub fn new() -> Self {
        PixelFifo {
            fetch_state: FetchState::Tile,
            fifo: VecDeque::new(),
            line_x: 0,
            pushed_x: 0,
            fetch_x: 0,
            bgw_fetch_data: [0; 3],
            fetch_entry_data: [0; 6],
            map_y: 0,
            map_x: 0,
            tile_y: 0,
            fifo_x: 0,
        }
    }
}

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
pub const YRES: usize = 144;
pub const XRES: usize = 160;
// Target frame rate is 60 Hz
const TARGET_FRAME_TIME: Duration = Duration::from_millis(16);

// window_line window line to draw
pub struct PPU {
    oam_ram: [Sprite; OAM_SIZE / 4],
    vram: [u8; VRAM_SIZE], // 8KB
    lcd: LCD,
    timer: Instant,
    start_time: Duration,
    prev_frame_time: Duration,
    frame_count: u32,
    current_frame: u32,
    line_ticks: u32,
    video_buffer: [u32; YRES * XRES],
    pixel_fifo: PixelFifo,
    line_sprites: VecDeque<Sprite>,
    fetched_entries: Vec<Sprite>,
    window_line: u8,
}

impl PPU {
    pub fn new() -> Self {
        let mut lcd = LCD::new();
        lcd.set_mode(LcdMode::OAM);

        PPU {
            oam_ram: core::array::from_fn(|_| Sprite::new()),
            vram: [0; VRAM_SIZE],
            lcd,
            timer: Instant::now(),
            start_time: Duration::from_millis(0),
            prev_frame_time: Duration::from_millis(0),
            frame_count: 0,
            current_frame: 0,
            line_ticks: 0,
            video_buffer: [0; YRES * XRES],
            pixel_fifo: PixelFifo::new(),
            line_sprites: VecDeque::new(),
            fetched_entries: Vec::new(),
            window_line: 0,
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

        let sprite_index = oam_address / 4;
        let sprite_field = oam_address % 4;
        let sprite = &self.oam_ram[sprite_index];

        match sprite_field {
            0 => sprite.y,
            1 => sprite.x,
            2 => sprite.tile_index,
            3 => sprite.flags.bits(),
            _ => panic!("Invalid sprite field index {sprite_field}"),
        }
    }

    pub fn oam_write(&mut self, address: u16, value: u8) {
        let oam_address = if address >= 0xFE00 {
            (address - 0xFE00) as usize
        } else {
            address as usize
        };

        let sprite_index = oam_address / 4;
        let sprite_field = oam_address % 4;
        let sprite = &mut self.oam_ram[sprite_index];

        match sprite_field {
            0 => sprite.y = value,
            1 => sprite.x = value,
            2 => sprite.tile_index = value,
            3 => sprite.flags = SpriteFlags::from_bits_truncate(value),
            _ => panic!("Invalid sprite field index {sprite_field}"),
        };
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

    pub fn video_buffer_read(&self, pixel_index: usize) -> u32 {
        self.video_buffer[pixel_index]
    }

    pub fn tick<I: InterruptRequest>(&mut self, ctx: &mut I) {
        self.line_ticks += 1;
        let lcd_mode = self.lcd.get_mode();

        match lcd_mode {
            LcdMode::OAM => self.tick_oam(),
            LcdMode::XFER => self.tick_xfer(ctx),
            LcdMode::VBLANK => self.tick_vblank(ctx),
            LcdMode::HBLANK => self.tick_hblank(ctx),
        }
    }

    fn load_line_sprites(&mut self) {
        let ly = self.lcd.ly;
        let sprite_height = self.lcd.get_sprite_height();

        for sprite in &self.oam_ram {
            if sprite.x == 0 {
                // Not visible
                continue;
            }

            if self.line_sprites.len() >= 10 {
                // Max 10 sprites per line
                break;
            }

            if sprite.y <= (ly + 16) && (sprite.y + sprite_height) > (ly + 16) {
                // This sprite is on the current line

                if self.line_sprites.is_empty() || self.line_sprites.front().unwrap().x > sprite.x {
                    self.line_sprites.push_front(sprite.clone());
                    continue;
                }

                for i in 0..self.line_sprites.len() {
                    if self.line_sprites[i].x > sprite.x {
                        self.line_sprites.insert(i, sprite.clone());
                    }
                }
            }
        }
    }

    fn tick_oam(&mut self) {
        if self.line_ticks >= 80 {
            self.lcd.set_mode(LcdMode::XFER);

            self.pixel_fifo.fetch_state = FetchState::Tile;
            self.pixel_fifo.line_x = 0;
            self.pixel_fifo.fetch_x = 0;
            self.pixel_fifo.pushed_x = 0;
            self.pixel_fifo.fifo_x = 0;
        }

        if self.line_ticks == 1 {
            // Read all sprites on the first tick, not as in hardware
            self.line_sprites.clear();
            self.load_line_sprites();
        }
    }

    fn tick_xfer<I: InterruptRequest>(&mut self, ctx: &mut I) {
        self.pipeline_process();

        if (self.pixel_fifo.pushed_x as usize) >= XRES {
            self.pixel_fifo.fifo.clear(); // Reset pixel FIFO

            self.lcd.set_mode(LcdMode::HBLANK);

            if self.lcd.lcds.contains(LcdStatus::HBLANK_INT_SELECT) {
                ctx.request_interrupt(InterruptFlag::LCD);
            }
        }
    }

    fn tick_vblank<I: InterruptRequest>(&mut self, ctx: &mut I) {
        if self.line_ticks >= TICKS_PER_LINE {
            self.increment_ly(ctx);

            if (self.lcd.ly as u32) >= LINES_PER_FRAME {
                self.lcd.set_mode(LcdMode::OAM);
                self.lcd.ly = 0;
                self.window_line = 0;
            }

            self.line_ticks = 0;
        }
    }

    fn tick_hblank<I: InterruptRequest>(&mut self, ctx: &mut I) {
        if self.line_ticks >= TICKS_PER_LINE {
            self.increment_ly(ctx);

            if (self.lcd.ly as usize) >= YRES {
                self.lcd.set_mode(LcdMode::VBLANK);

                ctx.request_interrupt(InterruptFlag::VBLANK);

                if self.lcd.lcds.contains(LcdStatus::VBLANK_INT_SELECT) {
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

    fn pipeline_process(&mut self) {
        self.pixel_fifo.map_y = self.lcd.ly + self.lcd.scroll_y;
        self.pixel_fifo.map_x = self.pixel_fifo.fetch_x + self.lcd.scroll_x;
        self.pixel_fifo.tile_y = ((self.lcd.ly + self.lcd.scroll_y) % 8) * 2;

        if (self.line_ticks & 1) == 0 {
            // Even line
            self.pipeline_fetch();
        }

        self.pipeline_push_pixel();
    }

    fn pipeline_load_sprite_tile(&mut self) {
        for entry in &self.line_sprites {
            let sp_x = (entry.x - 8) + (self.lcd.scroll_x % 8);

            if (sp_x >= self.pixel_fifo.fetch_x && sp_x < (self.pixel_fifo.fetch_x + 8))
                || ((sp_x + 8) >= self.pixel_fifo.fetch_x
                    && (sp_x + 8) < (self.pixel_fifo.fetch_x + 8))
            {
                self.fetched_entries.push(entry.clone());
            }

            if self.fetched_entries.len() >= 3 {
                // Max checking 3 sprites per pixel
                break;
            }
        }
    }

    fn pipeline_load_sprite_data(&mut self, offset: usize) {
        let ly = self.lcd.ly;
        let sprite_height = self.lcd.get_sprite_height();

        for i in 0..self.fetched_entries.len() {
            let entry = &self.fetched_entries[i];
            let mut ty = ((ly + 16) - entry.y) * 2;

            if entry.flags.contains(SpriteFlags::Y_FLIP) {
                ty = (2 * sprite_height - 2) - ty;
            }

            let mut tile_index = entry.tile_index as u16;

            if sprite_height == 16 {
                tile_index &= !1; // Remove last bit
            }

            let address = 0x8000 + (tile_index * 16) + (ty as u16) + (offset as u16);

            self.pixel_fifo.fetch_entry_data[(i * 2) + offset] = self.vram_read(address);
        }
    }

    fn pipeline_load_window_tile(&mut self) {
        if !self.lcd.is_window_visible() {
            return;
        }

        if (self.pixel_fifo.fetch_x + 7) >= self.lcd.win_x
            && (self.pixel_fifo.fetch_x + 7) < (self.lcd.win_x + (YRES as u8) + 14)
            && self.lcd.ly >= self.lcd.win_y
            && self.lcd.ly < (self.lcd.win_y + (XRES as u8))
        {
            let window_tile_y = (self.window_line as u16) / 8;
            let address = self.lcd.get_win_map_area()
                + (((self.pixel_fifo.fetch_x + 7 - self.lcd.win_x) / 8) as u16)
                + (window_tile_y * 32);
            self.pixel_fifo.bgw_fetch_data[0] = self.vram_read(address);

            if self.lcd.get_bgw_data_area() == 0x8800 {
                // Load from the second tile set data
                // Here we convert from negative to positive indices, -128 is 0
                self.pixel_fifo.bgw_fetch_data[0] =
                    self.pixel_fifo.bgw_fetch_data[0].wrapping_add(128);
            }
        }
    }

    fn pipeline_fetch(&mut self) {
        match self.pixel_fifo.fetch_state {
            FetchState::Tile => {
                self.fetched_entries.clear();

                if self.lcd.lcdc.contains(LcdControl::BG_WINDOW_ENABLE) {
                    let address = self.lcd.get_bg_map_area()
                        + ((self.pixel_fifo.map_x as u16) / 8)
                        + (((self.pixel_fifo.map_y as u16) / 8) * 32);
                    self.pixel_fifo.bgw_fetch_data[0] = self.vram_read(address);

                    if self.lcd.get_bgw_data_area() == 0x8800 {
                        // Load from the second tile set data
                        // Here we convert from negative to positive indices, -128 is 0
                        self.pixel_fifo.bgw_fetch_data[0] =
                            self.pixel_fifo.bgw_fetch_data[0].wrapping_add(128);
                    }

                    self.pipeline_load_window_tile();
                }

                if self.lcd.lcdc.contains(LcdControl::OBJ_ENABLE) && !self.line_sprites.is_empty() {
                    self.pipeline_load_sprite_tile();
                }

                self.pixel_fifo.fetch_state = FetchState::DataLow;
                self.pixel_fifo.fetch_x += 8;
            }
            FetchState::DataLow => {
                let address = self.lcd.get_bgw_data_area()
                    + ((self.pixel_fifo.bgw_fetch_data[0] as u16) * 16)
                    + (self.pixel_fifo.tile_y as u16);
                self.pixel_fifo.bgw_fetch_data[1] = self.vram_read(address);

                self.pipeline_load_sprite_data(0);

                self.pixel_fifo.fetch_state = FetchState::DataHigh;
            }
            FetchState::DataHigh => {
                let address = self.lcd.get_bgw_data_area()
                    + ((self.pixel_fifo.bgw_fetch_data[0] as u16) * 16)
                    + (self.pixel_fifo.tile_y as u16)
                    + 1;
                self.pixel_fifo.bgw_fetch_data[2] = self.vram_read(address);

                self.pipeline_load_sprite_data(1);

                self.pixel_fifo.fetch_state = FetchState::Idle;
            }
            FetchState::Idle => {
                self.pixel_fifo.fetch_state = FetchState::Push;
            }
            FetchState::Push => {
                if self.pipeline_fifo_add() {
                    self.pixel_fifo.fetch_state = FetchState::Tile;
                }
            }
        }
    }

    fn pipeline_push_pixel(&mut self) {
        if self.pixel_fifo.fifo.len() > 8 {
            // 8 pixels are required for the Pixel Rendering operation to take place
            let pixel_data = self.pixel_fifo.fifo.pop_front().unwrap();

            if self.pixel_fifo.line_x >= (self.lcd.scroll_x % 8) {
                let pixel_index =
                    (self.pixel_fifo.pushed_x as usize) + ((self.lcd.ly as usize) * XRES);
                self.video_buffer[pixel_index] = pixel_data;
                self.pixel_fifo.pushed_x += 1;
            }

            self.pixel_fifo.line_x += 1;
        }
    }

    fn pipeline_fifo_add(&mut self) -> bool {
        if self.pixel_fifo.fifo.len() > 8 {
            // Pixel FIFO is full
            return false;
        }

        let x = (self.pixel_fifo.fetch_x as i32) - (8 - ((self.lcd.scroll_x as i32) % 8));

        for i in 0..8 {
            let bit = 7 - i;
            let lo = ((self.pixel_fifo.bgw_fetch_data[1] & (1 << bit)) != 0) as u8;
            let hi = ((self.pixel_fifo.bgw_fetch_data[2] & (1 << bit)) != 0) as u8;
            let color_index = ((hi << 1) | lo) as usize;
            let mut color = self.lcd.bg_colors[color_index];

            if !self.lcd.lcdc.contains(LcdControl::BG_WINDOW_ENABLE) {
                color = self.lcd.bg_colors[0];
            }

            if self.lcd.lcdc.contains(LcdControl::OBJ_ENABLE) {
                color = self.fetch_sprite_pixels(color_index, color);
            }

            if x >= 0 {
                self.pixel_fifo.fifo.push_back(color);
                self.pixel_fifo.fifo_x += 1;
            }
        }

        true
    }

    fn fetch_sprite_pixels(&self, bg_color_index: usize, default_color: u32) -> u32 {
        let mut color = default_color;
        for i in 0..self.fetched_entries.len() {
            let entry = &self.fetched_entries[i];
            let sp_x = (entry.x - 8) + (self.lcd.scroll_x % 8);

            if (sp_x + 8) < self.pixel_fifo.fifo_x {
                // Passed pixel point already
                continue;
            }
            // TODO: Is wrapping_sub correct?
            let offset = self.pixel_fifo.fifo_x.wrapping_sub(sp_x);

            if offset > 7 {
                // Out of bounds
                continue;
            }

            let mut bit = 7 - offset;

            if entry.flags.contains(SpriteFlags::X_FLIP) {
                bit = offset;
            }

            let lo = ((self.pixel_fifo.fetch_entry_data[i * 2] & (1 << bit)) != 0) as u8;
            let hi = ((self.pixel_fifo.fetch_entry_data[i * 2 + 1] & (1 << bit)) != 0) as u8;
            let color_index = ((hi << 1) | lo) as usize;
            let bg_priority = entry.flags.contains(SpriteFlags::PRIORITY);

            if color_index == 0 {
                // Transparent
                continue;
            }

            if !bg_priority || bg_color_index == 0 {
                color = if entry.flags.contains(SpriteFlags::DMG_PALETTE) {
                    self.lcd.sp1_colors[color_index]
                } else {
                    self.lcd.sp0_colors[color_index]
                };

                break;
            }
        }

        color
    }

    pub fn increment_ly<I: InterruptRequest>(&mut self, ctx: &mut I) {
        if self.lcd.is_window_visible()
            && self.lcd.ly >= self.lcd.win_y
            && self.lcd.ly < (self.lcd.win_y + (YRES as u8))
        {
            self.window_line += 1;
        }

        self.lcd.ly = self.lcd.ly.wrapping_add(1);

        if self.lcd.ly == self.lcd.lyc {
            self.lcd.lcds.insert(LcdStatus::LYC_EQUAL_LY);

            if self.lcd.lcds.contains(LcdStatus::LYC_INT_SELECT) {
                ctx.request_interrupt(InterruptFlag::LCD);
            }
        } else {
            self.lcd.lcds.remove(LcdStatus::LYC_EQUAL_LY);
        }
    }
}

impl Default for PPU {
    fn default() -> Self {
        PPU::new()
    }
}

#[derive(Clone)]
struct Sprite {
    y: u8,
    x: u8,
    tile_index: u8,
    flags: SpriteFlags,
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y: 0,
            x: 0,
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
