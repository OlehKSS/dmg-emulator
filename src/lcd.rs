use super::bus::HardwareRegister;
use super::interrupts::{InterruptFlag, InterruptRequest};
use bitflags::bitflags;

pub static DEFAULT_COLORS: [u32; 4] = [0xFFFFFFFF, 0xFFAAAAAA, 0xFF555555, 0xFF000000];

bitflags!(
    pub struct LcdControl : u8 {
        const LCD_PPU_ENABLE = 0b1000_0000;
        const WINDOW_TILE_MAP_AREA = 0b0100_0000;
        const WINDOW_ENABLE = 0b0010_0000;
        const BG_WINDOW_TILE_DATA_AREA = 0b0001_0000;
        const BG_TILE_MAP_AREA = 0b0000_1000;
        const OBJ_SIZE = 0b0000_0100;
        const OBJ_ENABLE = 0b0000_0010;
        const BG_WINDOW_ENABLE = 0b0000_0001;
    }
);

bitflags!(
/// LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt.
///
/// Mode (0=HBLANK, 1=VBLANK, 2=OAM) int select (Read/Write):
/// If set, selects the Mode 2 (1 or 0) condition for the STAT interrupt.
///
/// LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
///
/// PPU mode (Read-only): Indicates the PPU’s current status (0=HBLANK, 1=VBLANK, 2=OAM, 3=XFER).
/// Reports 0 instead when the PPU is disabled.
    pub struct LcdStatus: u8 {
        const LYC_INT_SELECT = 0b0100_0000;
        const OAM_INT_SELECT = 0b0010_0000;
        const VBLANK_INT_SELECT = 0b0001_0000;
        const HBLANK_INT_SELECT = 0b0000_1000;
        const LYC_EQUAL_LY = 0b0000_0100;
        const PPU_MODE = 0b0000_0011;
    }
);

pub struct LCD {
    lcdc: LcdControl,
    lcds: LcdStatus,
    scroll_x: u8,
    scroll_y: u8,
    pub ly: u8,
    pub lyc: u8,
    dma: u8,
    bg_palette: u8,
    obj_palette: [u8; 2],
    win_x: u8,
    win_y: u8,

    bg_colors: [u32; 4],
    sp0_colors: [u32; 4],
    sp1_colors: [u32; 4],
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum LcdMode {
    HBLANK,
    VBLANK,
    OAM,
    XFER,
}

impl From<u8> for LcdMode {
    fn from(value: u8) -> Self {
        match value {
            0 => LcdMode::HBLANK,
            1 => LcdMode::VBLANK,
            2 => LcdMode::OAM,
            3 => LcdMode::XFER,
            _ => panic!("Invalid LcdMode value"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Palette {
    Background,
    Object0,
    Object1,
}

impl Default for LCD {
    fn default() -> Self {
        LCD::new()
    }
}

impl LCD {
    pub fn new() -> Self {
        LCD {
            lcdc: LcdControl::from_bits_truncate(0x91),
            lcds: LcdStatus::from_bits_truncate(0),
            scroll_x: 0,
            scroll_y: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bg_palette: 0xFC,
            obj_palette: [0xFF, 0xFF],
            win_x: 0,
            win_y: 0,
            bg_colors: DEFAULT_COLORS,
            sp0_colors: DEFAULT_COLORS,
            sp1_colors: DEFAULT_COLORS,
        }
    }

    pub fn get_mode(&self) -> LcdMode {
        let mode = self.lcds.bits() & LcdStatus::PPU_MODE.bits();
        LcdMode::from(mode)
    }

    pub fn set_mode(&mut self, mode: LcdMode) {
        // reset PPU_MODE and set its new value
        self.lcds.remove(LcdStatus::PPU_MODE);
        self.lcds = LcdStatus::from_bits_truncate(self.lcds.bits() | mode as u8);
        // TODO: Remove this check later
        assert!(self.get_mode() == mode);
    }

    pub fn status_contains(&self, other: LcdStatus) -> bool {
        self.lcds.contains(other)
    }

    pub fn increment_ly<I: InterruptRequest>(&mut self, ctx: &mut I) {
        self.ly = self.ly.wrapping_add(1);

        if self.ly == self.lyc {
            self.lcds.insert(LcdStatus::LYC_EQUAL_LY);

            if self.lcds.contains(LcdStatus::LYC_INT_SELECT) {
                ctx.request_interrupt(InterruptFlag::LCD);
            }
        } else {
            self.lcds.remove(LcdStatus::LYC_EQUAL_LY);
        }
    }

    pub fn read(&self, address: HardwareRegister) -> u8 {
        match address {
            HardwareRegister::LCDC => self.lcdc.bits(),
            HardwareRegister::STAT => self.lcds.bits(),
            HardwareRegister::SCY => self.scroll_y,
            HardwareRegister::SCX => self.scroll_x,
            HardwareRegister::LY => self.ly,
            HardwareRegister::LYC => self.lyc,
            HardwareRegister::DMA => self.dma,
            HardwareRegister::BGP => self.bg_palette,
            HardwareRegister::OBP0 => self.obj_palette[0],
            HardwareRegister::OBP1 => self.obj_palette[1],
            HardwareRegister::WY => self.win_y,
            HardwareRegister::WX => self.win_x,
            _ => panic!("Invalid LCD register 0x{:04X}.", address as u8),
        }
    }

    pub fn write(&mut self, address: HardwareRegister, value: u8) {
        match address {
            HardwareRegister::LCDC => self.lcdc = LcdControl::from_bits_truncate(value),
            HardwareRegister::STAT => self.lcds = LcdStatus::from_bits_truncate(value),
            HardwareRegister::SCY => self.scroll_y = value,
            HardwareRegister::SCX => self.scroll_x = value,
            HardwareRegister::LY => self.ly = value,
            HardwareRegister::LYC => self.lyc = value,
            HardwareRegister::DMA => {
                panic!("DMA start not implemented")
            }
            HardwareRegister::BGP => {
                self.bg_palette = value;
                self.update_palette(Palette::Background, value);
            }
            HardwareRegister::OBP0 => {
                self.obj_palette[0] = value;
                self.update_palette(Palette::Object0, value & 0b11111100);
            }
            HardwareRegister::OBP1 => {
                self.obj_palette[1] = value;
                self.update_palette(Palette::Object1, value & 0b11111100);
            }
            HardwareRegister::WY => self.win_y = value,
            HardwareRegister::WX => self.win_x = value,
            _ => panic!("Invalid LCD register 0x{:04X}.", address as u8),
        }
    }

    fn update_palette(&mut self, palette: Palette, color_indices: u8) {
        let colors = match palette {
            Palette::Background => &mut self.bg_colors,
            Palette::Object0 => &mut self.sp0_colors,
            Palette::Object1 => &mut self.sp1_colors,
        };

        colors[0] = DEFAULT_COLORS[(color_indices & 0b11) as usize];
        colors[1] = DEFAULT_COLORS[((color_indices >> 2) & 0b11) as usize];
        colors[2] = DEFAULT_COLORS[((color_indices >> 4) & 0b11) as usize];
        colors[3] = DEFAULT_COLORS[((color_indices >> 6) & 0b11) as usize];
    }
}
