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
// 0xFF80 - 0xFFFE : Zero Page or High RAM
// 0xFFFF: Interrupt Enabled Register
#[derive(Debug)]
pub struct MemoryBus {
    bytes: [u8; 0xFFFF + 1],
    rom: Option<Cartridge>,
}

/// P1/JOYP Joypad
/// SB Serial transfer data
/// SC Serial transfer control
/// DIV Divider register
/// TIMA Timer counter
/// TMA Timer modulo
/// TAC Timer control
/// IF Interrupt flag
/// IE Interrupt enable
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HardwareRegister {
    P1_JOYP = 0xFF00,
    SB = 0xFF01,
    SC = 0xFF02,
    DIV = 0xFF04,
    TIMA = 0xFF05,
    TMA = 0xFF06,
    TAC = 0xFF07,
    IF = 0xFF0F,
    IE = 0xFFFF,
}

impl HardwareRegister {
    pub fn from_u16(address: u16) -> Option<HardwareRegister> {
        match address {
            x if x == HardwareRegister::P1_JOYP as u16 => Some(HardwareRegister::P1_JOYP),
            x if x == HardwareRegister::SB as u16 => Some(HardwareRegister::SB),
            x if x == HardwareRegister::SC as u16 => Some(HardwareRegister::SC),
            x if x == HardwareRegister::DIV as u16 => Some(HardwareRegister::DIV),
            x if x == HardwareRegister::TIMA as u16 => Some(HardwareRegister::TIMA),
            x if x == HardwareRegister::TMA as u16 => Some(HardwareRegister::TMA),
            x if x == HardwareRegister::TAC as u16 => Some(HardwareRegister::TAC),
            x if x == HardwareRegister::IF as u16 => Some(HardwareRegister::IF),
            x if x == HardwareRegister::IE as u16 => Some(HardwareRegister::IE),
            _ => None,
        }
    }
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBus {
    pub fn new() -> Self {
        MemoryBus {
            bytes: [0; 0xFFFF + 1],
            rom: None,
        }
    }

    pub fn from_rom(rom: Option<Cartridge>) -> Self {
        MemoryBus {
            bytes: [0; 0xFFFF + 1],
            rom,
        }
    }

    pub fn set_rom(&mut self, rom: Option<Cartridge>) {
        self.rom = rom;
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0..=0x7FFF => self.rom.as_ref().unwrap().data[address as usize],
            0x8000..=0x9FFF => self.bytes[address as usize],
            0xA000..=0xBFFF => self.rom.as_ref().unwrap().data[address as usize],
            0xC000..=0xCFFF => self.bytes[address as usize],
            0xD000..=0xDFFF => {
                // In DMG mode, 0xD000 - 0xDFFF mirrors 0xC000 - 0xCFFF (RAM Bank 0).
                let rom0_address = address - 0x1000;
                self.bytes[rom0_address as usize]
            }
            0xE000..=0xFDFF => {
                // Reserved, echo RAM
                0
            }
            0xFE00..=0xFE9F => self.bytes[address as usize],
            0xFEA0..=0xFEFF => {
                // Reserved, unusable
                0
            }
            0xFF00..=0xFF7F => self.bytes[address as usize],
            0xFF80..=0xFFFE => self.bytes[address as usize],
            0xFFFF => self.bytes[address as usize],
        }
    }

    pub fn read16(&self, address: u16) -> u16 {
        let lo = self.read(address) as u16;
        let hi = self.read(address + 1) as u16;
        lo | (hi << 8)
    }

    pub fn read_register(&self, register: HardwareRegister) -> u8 {
        let address = register as u16;
        self.read(address)
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

    pub fn write_register(&mut self, register: HardwareRegister, value: u8) {
        let address = register as u16;
        self.write(address, value);
    }
}
