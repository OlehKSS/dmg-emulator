use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use super::lcd::DEFAULT_COLORS;
use super::ppu::PPU;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiAction {
    Exit,
    Continue,
}

#[allow(dead_code)]
pub struct GUI {
    sdl_context: sdl2::Sdl,
    // Canvas to keeps windows open
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    debug_canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
}

impl Default for GUI {
    fn default() -> Self {
        GUI::new(false)
    }
}

impl GUI {
    const SCREEN_WIDTH: u32 = 20;
    const SCREEN_HEIGHT: u32 = 18;
    const DEBUG_SCREEN_WIDTH: u32 = 16;
    const DEBUG_SCREEN_HEIGHT: u32 = 24;
    const SCALE: u32 = 5;

    pub fn new(debug: bool) -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
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

        if debug {
            let debug_window = video_subsystem
                .window(
                    "Debug Info",
                    Self::DEBUG_SCREEN_WIDTH * 8 * Self::SCALE
                        + Self::DEBUG_SCREEN_WIDTH * Self::SCALE,
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
            debug_canvas.set_draw_color(Color::RGB(0, 0, 0));
            debug_canvas.clear();
            debug_canvas.present();

            return GUI {
                sdl_context,
                canvas,
                debug_canvas: Some(debug_canvas),
            };
        }

        GUI {
            sdl_context,
            canvas,
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
