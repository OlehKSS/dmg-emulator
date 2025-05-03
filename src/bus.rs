use super::cart::Cartridge;

// 0x0000 - 0x3FFF : ROM Bank 0
// 0x4000 - 0x7FFF : ROM Bank 1 - Switchable
// 0x8000 - 0x97FF : CHR RAM
// 0x9800 - 0x9BFF : BG Map 1
// 0x9C00 - 0x9FFF : BG Map 2
// 0xA000 - 0xBFFF : Cartridge RAM
// 0xC000 - 0xCFFF : RAM Bank 0
// 0xD000 - 0xDFFF : RAM Bank 1-7 - switchable - Color only
// 0xE000 - 0xFDFF : Reserved - Echo RAM
// 0xFE00 - 0xFE9F : Object Attribute Memory
// 0xFEA0 - 0xFEFF : Reserved - Unusable
// 0xFF00 - 0xFF7F : I/O Registers
// 0xFF80 - 0xFFFE : Zero Page
// 0xFFFF: Interrupt Enabled Register
#[derive(Debug)]
pub struct MemoryBus {
    bytes: [u8; 0xFFFF],
    rom: Cartridge,
}

impl MemoryBus {
    pub fn new(rom: Cartridge) -> Self {
        MemoryBus {
            bytes: [0; 0xFFFF],
            rom,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0..=0x7FFF => self.rom.data[address as usize],
            0x8000..=0x9FFF => {
                // TODO: Char/Map data
                todo!(
                    "Not implemented reading Char/Map data from memory bus for address 0x{address:04X}"
                );
            }
            0xA000..=0xBFFF => self.rom.data[address as usize],
            0xC000..=0xDFFF => {
                todo!(
                    "Not implemented reading working RAM data from memory bus for address 0x{address:04X}"
                );
            }
            0xE000..=0xFDFF => {
                // Reserved, echo RAM
                0
            }
            0xFE00..=0xFE9F => {
                todo!(
                    "Not implemented reading Object Attribute Memory from memory bus for address 0x{address:04X}"
                )
            }
            0xFEA0..=0xFEFF => {
                // Reserved, unusable
                0
            }
            0xFF00..=0xFF7F => {
                todo!(
                    "Not implemented reading I/O registers from memory bus for address 0x{address:04X}"
                );
            }
            0xFF80..=0xFFFE => {
                todo!("Not implemented reading high RAM (zero page) for address 0x{address:04X}");
            }
            0xFFFF => {
                todo!(
                    "Not implemented reading Interrupt Enabled Register from memory bus for address 0x{address:04X}"
                );
            }
        }
    }

    pub fn read16(&self, address: u16) -> u16 {
        let lo = self.read(address) as u16;
        let hi = self.read(address + 1) as u16;
        lo | (hi << 8)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.bytes[address as usize] = value;
    }

    pub fn write16(&mut self, address: u16, value: u16) {
        let lo = (value & 0x00FF) as u8;
        let hi = ((value >> 8) & 0x00FF) as u8;
        self.bytes[address as usize] = lo;
        self.bytes[(address + 1) as usize] = hi;
    }
}
