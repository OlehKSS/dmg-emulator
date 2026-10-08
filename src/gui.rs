use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};

use super::lcd::DEFAULT_COLORS;
use super::ppu::{PPU, XRES, YRES};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiAction {
    Exit,
    Continue,
}

#[allow(dead_code)]
pub struct GUI {
    sdl_context: sdl2::Sdl,
    // Canvas to keeps windows open
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    debug_canvas: Option<Canvas<Window>>,
}

impl Default for GUI {
    fn default() -> Self {
        GUI::new(false)
    }
}

impl GUI {
    const DEBUG_SCREEN_WIDTH: u32 = 16;
    const DEBUG_SCREEN_HEIGHT: u32 = 24;
    const SCALE: u32 = 4;

    pub fn new(debug: bool) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window(
                "GameBoy Emulator",
                (XRES as u32) * Self::SCALE,
                (YRES as u32) * Self::SCALE,
            )
            .position_centered()
            .build()
            .unwrap();

        let (posx, posy) = window.position();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let texture_creator = canvas.texture_creator();

        if debug {
            let debug_window = video_subsystem
                .window(
                    "Debug Info",
                    Self::DEBUG_SCREEN_WIDTH * 9 * Self::SCALE,
                    Self::DEBUG_SCREEN_HEIGHT * 9 * Self::SCALE,
                )
                .position(posx + (((XRES as u32) * Self::SCALE) as i32) + 20, posy)
                .build()
                .unwrap();

            let mut debug_canvas = debug_window.into_canvas().build().unwrap();
            debug_canvas.set_draw_color(Color::RGB(0, 0, 0));
            debug_canvas.clear();
            debug_canvas.present();

            return GUI {
                sdl_context,
                canvas,
                texture_creator,
                debug_canvas: Some(debug_canvas),
            };
        }

        GUI {
            sdl_context,
            canvas,
            texture_creator,
            debug_canvas: None,
        }
    }

    pub fn handle_events(&self) -> GuiAction {
        let mut event_pump = self.sdl_context.event_pump().unwrap();
        let mut gui_event = GuiAction::Continue;

        for event in event_pump.poll_iter() {
            gui_event = match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => GuiAction::Exit,
                _ => GuiAction::Continue,
            };
        }

        gui_event
    }

    pub fn update_window(&mut self, video_buffer: &[u32; XRES * YRES]) {
        let mut texture = self
            .texture_creator
            .create_texture_streaming(PixelFormatEnum::ARGB8888, XRES as u32, YRES as u32)
            .unwrap();

        let mut pixel_data = [0u8; XRES * YRES * 4];

        for (i, color) in video_buffer.iter().enumerate() {
            let offset = i * 4;
            pixel_data[offset] = (color & 0xFF) as u8; // b
            pixel_data[offset + 1] = ((color >> 8) & 0xFF) as u8; // g
            pixel_data[offset + 2] = ((color >> 16) & 0xFF) as u8; // r
            pixel_data[offset + 3] = ((color >> 24) & 0xFF) as u8; // a
        }

        texture.update(None, &pixel_data, XRES * 4).unwrap();
        self.canvas.clear();
        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
    }

    pub fn update_debug_window(&mut self, ppu: &PPU) {
        if self.debug_canvas.is_none() {
            return;
        }

        let mut x_draw = 0i32;
        let mut y_draw = 0i32;
        let mut tile_num = 0u16;
        let scale = Self::SCALE as i32;

        for y in 0..Self::DEBUG_SCREEN_HEIGHT {
            for x in 0..Self::DEBUG_SCREEN_WIDTH {
                let x_tile = x_draw + ((x as i32) * scale);
                let y_tile = y_draw + ((y as i32) * scale);
                self.display_tile(ppu, tile_num, x_tile, y_tile);
                x_draw += 8 * scale;
                tile_num += 1;
            }
            y_draw += 8 * scale;
            x_draw = 0;
        }

        self.debug_canvas.as_mut().unwrap().present();
    }

    fn display_tile(&mut self, ppu: &PPU, tile_num: u16, x: i32, y: i32) {
        const START_ADDRESS: u16 = 0x8000;
        let scale = Self::SCALE as i32;

        for tile_byte in (0..16u16).step_by(2) {
            let b1 = ppu.vram_read(START_ADDRESS + tile_num * 16 + tile_byte);
            let b2 = ppu.vram_read(START_ADDRESS + tile_num * 16 + tile_byte + 1);

            for bit in (0..=7u16).rev() {
                let hi = ((b1 & (1 << bit)) != 0) as u8;
                let lo = ((b2 & (1 << bit)) != 0) as u8;
                let color_index = ((hi << 1) | lo) as usize;
                let color = color_from_u32(DEFAULT_COLORS[color_index]);

                let x_rc = x + (((7 - bit) as i32) * scale);
                let y_rc = y + (tile_byte as i32) / 2 * scale;
                let rc = Rect::new(x_rc, y_rc, Self::SCALE, Self::SCALE);

                self.debug_canvas.as_mut().unwrap().set_draw_color(color);
                self.debug_canvas.as_mut().unwrap().fill_rect(rc).unwrap();
            }
        }
    }
}

// Convert from ARGB to SDL2::Color
fn color_from_u32(color: u32) -> Color {
    let a = ((color >> 24) & 0xFF) as u8;
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;

    Color::RGBA(r, g, b, a)
}
