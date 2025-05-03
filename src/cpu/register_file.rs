use bitflags::bitflags;
use std::fmt;

bitflags!(
    /// The flags register is the lower 8 bits of the `AF` register and
    /// contains the following flags:
    ///
    /// - **Z (Zero flag)**: Set if the result of an operation is zero.
    /// - **N (Subtraction flag)**: Set if the last operation was a subtraction
    ///   (used for BCD arithmetic).
    /// - **H (Half Carry flag)**: Set if there was a carry from bit 3 to bit 4
    ///   in the result (used for BCD arithmetic).
    /// - **C (Carry flag)**: Set if there was a carry from the most significant
    ///   bit in the result.
    pub struct Flags: u8 {
    const ZERO         = 0b_1000_0000;
    const SUBTRACT = 0b_0100_0000;
    const HALF_CARRY   = 0b_0010_0000;
    const CARRY        = 0b_0001_0000;
    }
);

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Register {
    A = 0,
    F = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    H = 6,
    L = 7,
    AF = 8,
    BC = 9,
    DE = 10,
    HL = 11,
    SP = 12,
    PC = 13,
}

pub struct RegisterFile {
    registers: [u8; 8],
    pub pc: u16,
    pub sp: u16,
    flags: Flags,
}

impl Register {
    pub fn is_16bit(&self) -> bool {
        match self {
            Register::A
            | Register::F
            | Register::B
            | Register::C
            | Register::D
            | Register::E
            | Register::H
            | Register::L => false,
            Register::AF
            | Register::BC
            | Register::DE
            | Register::HL
            | Register::PC
            | Register::SP => true,
        }
    }
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        let mut registers = [0; 8];
        registers[Register::A as usize] = 0x01;
        registers[Register::F as usize] = 0xB0;
        registers[Register::C as usize] = 0x13;
        registers[Register::E as usize] = 0xD8;
        registers[Register::H as usize] = 0x01;
        registers[Register::L as usize] = 0x4D;

        RegisterFile {
            registers,
            pc: 0x100,
            sp: 0xFFFE,
            flags: Flags::empty(),
        }
    }

    pub fn read8(&self, reg: Register) -> u8 {
        match reg {
            Register::A
            | Register::F
            | Register::B
            | Register::C
            | Register::D
            | Register::E
            | Register::H
            | Register::L => self.registers[reg as usize],
            _ => panic!("Invalid register, only u8 supported"),
        }
    }

    pub fn read16(&self, reg: Register) -> u16 {
        match reg {
            Register::AF => {
                ((self.registers[Register::A as usize] as u16) << 8)
                    | (self.registers[Register::F as usize] as u16)
            }
            Register::BC => {
                ((self.registers[Register::B as usize] as u16) << 8)
                    | (self.registers[Register::C as usize] as u16)
            }
            Register::DE => {
                ((self.registers[Register::D as usize] as u16) << 8)
                    | (self.registers[Register::E as usize] as u16)
            }
            Register::HL => {
                ((self.registers[Register::H as usize] as u16) << 8)
                    | (self.registers[Register::L as usize] as u16)
            }
            Register::PC => self.pc,
            Register::SP => self.sp,
            _ => panic!("Invalid register, only u16 supported"),
        }
    }

    pub fn write8(&mut self, reg: Register, value: u8) {
        match reg {
            Register::A
            | Register::F
            | Register::B
            | Register::C
            | Register::D
            | Register::E
            | Register::H
            | Register::L => self.registers[reg as usize] = value,
            _ => panic!("Invalid register, only u8 supported"),
        }
    }

    pub fn write16(&mut self, reg: Register, value: u16) {
        let lo = (value & 0x00FF) as u8;
        let hi = ((value & 0xFF00) >> 8) as u8;

        match reg {
            Register::AF => {
                self.registers[Register::A as usize] = hi;
                // TODO: We need to use registers as separate fields
                // At the moment flags and Register::F are two distinct entities, not good.
                self.registers[Register::F as usize] = lo & 0xF0; // Mask lower 4 bits;
                Flags::from_bits_truncate(lo);
            }
            Register::BC => {
                self.registers[Register::B as usize] = hi;
                self.registers[Register::C as usize] = lo;
            }
            Register::DE => {
                self.registers[Register::D as usize] = hi;
                self.registers[Register::E as usize] = lo;
            }
            Register::HL => {
                self.registers[Register::H as usize] = hi;
                self.registers[Register::L as usize] = lo;
            }
            Register::PC => self.pc = value,
            Register::SP => self.sp = value,
            _ => panic!("Invalid register, only u16 supported"),
        }
    }

    #[inline]
    /// Get Zero flag (Z).
    pub fn zf(&self) -> bool {
        self.flags.contains(Flags::ZERO)
    }

    #[inline]
    /// Get Subtract flag (N).
    pub fn nf(&self) -> bool {
        self.flags.contains(Flags::SUBTRACT)
    }

    #[inline]
    /// Get Half Carry flag (H).
    pub fn hf(&self) -> bool {
        self.flags.contains(Flags::HALF_CARRY)
    }

    #[inline]
    /// Get Carry flag (C).
    pub fn cf(&self) -> bool {
        self.flags.contains(Flags::CARRY)
    }

    #[inline]
    /// Insert the zero flag (Z) if value if true or remove when the value is false.
    pub fn set_zf(&mut self, value: bool) {
        self.flags.set(Flags::ZERO, value);
    }

    #[inline]
    /// Insert the subtract flag (N) if value if true or remove when the value is false.
    pub fn set_nf(&mut self, value: bool) {
        self.flags.set(Flags::SUBTRACT, value);
    }

    #[inline]
    /// Insert the half carry flag (H) if value if true or remove when the value is false.
    pub fn set_hf(&mut self, value: bool) {
        self.flags.set(Flags::HALF_CARRY, value);
    }

    #[inline]
    /// Insert the carry flag (C) if value if true or remove when the value is false.
    pub fn set_cf(&mut self, value: bool) {
        self.flags.set(Flags::CARRY, value);
    }
}

impl fmt::Display for RegisterFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PC: {:04x} SP: {:04x} \
            A: {:04x} F: {:04x} B: {:04x} C: {:04x} \
            D: {:04x} E: {:04x} H: {:04x} L: {:04x}",
            self.pc,
            self.sp,
            self.registers[Register::A as usize],
            self.registers[Register::F as usize],
            self.registers[Register::B as usize],
            self.registers[Register::C as usize],
            self.registers[Register::D as usize],
            self.registers[Register::E as usize],
            self.registers[Register::H as usize],
            self.registers[Register::L as usize],
        )
    }
}
