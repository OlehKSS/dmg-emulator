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
    pub a: u8,
    pub f: Flags,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
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
        RegisterFile {
            a: 0x01,
            f: Flags::from_bits_truncate(0xB0),
            b: 0,
            c: 0x13,
            d: 0,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            pc: 0x100,
            sp: 0xFFFE,
        }
    }

    pub fn read8(&self, reg: Register) -> u8 {
        match reg {
            Register::A => self.a,
            Register::F => self.f.bits(),
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::H => self.h,
            Register::L => self.l,
            _ => panic!("Invalid register, only u8 supported"),
        }
    }

    pub fn read16(&self, reg: Register) -> u16 {
        match reg {
            Register::AF => ((self.a as u16) << 8) | (self.f.bits() as u16),
            Register::BC => ((self.b as u16) << 8) | (self.c as u16),
            Register::DE => ((self.d as u16) << 8) | (self.e as u16),
            Register::HL => ((self.h as u16) << 8) | (self.l as u16),
            Register::PC => self.pc,
            Register::SP => self.sp,
            _ => panic!("Invalid register, only u16 supported"),
        }
    }

    pub fn write8(&mut self, reg: Register, value: u8) {
        match reg {
            Register::A => self.a = value,
            Register::F => self.f = Flags::from_bits_truncate(value),
            Register::B => self.b = value,
            Register::C => self.c = value,
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::H => self.h = value,
            Register::L => self.l = value,
            _ => panic!("Invalid register, only u8 supported"),
        }
    }

    pub fn write16(&mut self, reg: Register, value: u16) {
        let lo = (value & 0x00FF) as u8;
        let hi = ((value & 0xFF00) >> 8) as u8;

        match reg {
            Register::AF => {
                self.a = hi;
                self.f = Flags::from_bits_truncate(lo);
            }
            Register::BC => {
                self.b = hi;
                self.c = lo;
            }
            Register::DE => {
                self.d = hi;
                self.e = lo;
            }
            Register::HL => {
                self.h = hi;
                self.l = lo;
            }
            Register::PC => self.pc = value,
            Register::SP => self.sp = value,
            _ => panic!("Invalid register, only u16 supported"),
        }
    }

    #[inline]
    /// Get Zero flag (Z).
    pub fn zf(&self) -> bool {
        self.f.contains(Flags::ZERO)
    }

    #[inline]
    /// Get Subtract flag (N).
    pub fn nf(&self) -> bool {
        self.f.contains(Flags::SUBTRACT)
    }

    #[inline]
    /// Get Half Carry flag (H).
    pub fn hf(&self) -> bool {
        self.f.contains(Flags::HALF_CARRY)
    }

    #[inline]
    /// Get Carry flag (C).
    pub fn cf(&self) -> bool {
        self.f.contains(Flags::CARRY)
    }

    #[inline]
    /// Insert the zero flag (Z) if value if true or remove when the value is false.
    pub fn set_zf(&mut self, value: bool) {
        self.f.set(Flags::ZERO, value);
    }

    #[inline]
    /// Insert the subtract flag (N) if value if true or remove when the value is false.
    pub fn set_nf(&mut self, value: bool) {
        self.f.set(Flags::SUBTRACT, value);
    }

    #[inline]
    /// Insert the half carry flag (H) if value if true or remove when the value is false.
    pub fn set_hf(&mut self, value: bool) {
        self.f.set(Flags::HALF_CARRY, value);
    }

    #[inline]
    /// Insert the carry flag (C) if value if true or remove when the value is false.
    pub fn set_cf(&mut self, value: bool) {
        self.f.set(Flags::CARRY, value);
    }
}

impl fmt::Display for RegisterFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PC: {:04X} SP: {:04X} \
            A: {:04X} F: {:04X} B: {:04X} C: {:04X} \
            D: {:04X} E: {:04X} H: {:04X} L: {:04X}",
            self.pc, self.sp, self.a, self.f, self.d, self.c, self.d, self.e, self.h, self.l,
        )
    }
}
