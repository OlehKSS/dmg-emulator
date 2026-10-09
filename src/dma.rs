use super::bus::MemoryBus;
use super::ppu::PPU;

pub struct DMA {
    active: bool,
    byte: u8,
    start_delay: u8,
    value: u8,
}

impl DMA {
    pub fn new() -> Self {
        DMA {
            active: false,
            byte: 0,
            start_delay: 0,
            value: 0,
        }
    }

    pub fn start(&mut self, value: u8) {
        self.active = true;
        self.byte = 0;
        self.start_delay = 2;
        self.value = value;
    }

    pub fn tick_cycle(&mut self, bus: &MemoryBus, ppu: &mut PPU) {
        if !self.active {
            return;
        }

        if self.start_delay > 0 {
            self.start_delay -= 1;
            return;
        }

        let address = (self.value as u16) * 0x100 + (self.byte as u16);
        let oam_value = bus.read(address);
        ppu.oam_write(self.byte as u16, oam_value);

        self.byte += 1;
        self.active = self.byte < 0xA0; // Up to 160 bytes
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for DMA {
    fn default() -> Self {
        DMA::new()
    }
}
