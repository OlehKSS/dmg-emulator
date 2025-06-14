/// Represents the various instructions that can be executed by the emulator.
///
/// Each variant corresponds to a specific operation that can be performed,
/// such as arithmetic, bitwise, or control operations. Below is a brief
/// description of each instruction:
///
/// - `ADD`: Adds the value of a specific register to the A register.
/// - `ADDHL`: Adds the value of a specific register to the HL register.
/// - `ADC`: Adds the value of a specific register to the A register, including the carry flag.
/// - `SUB`: Subtracts the value of a specific register from the A register.
/// - `SBC`: Subtracts the value of a specific register from the A register, including the carry flag.
/// - `AND`: Performs a bitwise AND between the value of a specific register and the A register.
/// - `OR`: Performs a bitwise OR between the value of a specific register and the A register.
/// - `XOR`: Performs a bitwise XOR between the value of a specific register and the A register.
/// - `CP`: Compares the value of a specific register with the A register (like `SUB` but does not store the result).
/// - `INC`: Increments the value of a specific register by 1.
/// - `DEC`: Decrements the value of a specific register by 1.
/// - `CCF`: Toggles the value of the carry flag.
/// - `SFC`: Sets the carry flag to true.
/// - `RRA`: Rotates the A register right through the carry flag.
/// - `RLA`: Rotates the A register left through the carry flag.
/// - `RRCA`: Rotates the A register right (not through the carry flag).
/// - `RRLA`: Rotates the A register left (not through the carry flag).
/// - `CPL`: Toggles every bit of the A register.
/// - `BIT`: Tests if a specific bit of a specific register is set.
/// - `RESET`: Sets a specific bit of a specific register to 0.
/// - `SET`: Sets a specific bit of a specific register to 1.
/// - `SRL`: Performs a logical right shift on a specific register by 1.
/// - `RR`: Rotates a specific register right through the carry flag.
/// - `RL`: Rotates a specific register left through the carry flag.
/// - `RRC`: Rotates a specific register right (not through the carry flag).
/// - `RLC`: Rotates a specific register left (not through the carry flag).
/// - `SRA`: Performs an arithmetic right shift on a specific register by 1.
/// - `SLA`: Performs an arithmetic left shift on a specific register by 1.
/// - `SWAP`: Swaps the upper and lower nibbles of a specific register.
/// - `LDH`: Load a value to or from a specific memory address in the high RAM area (0xFF00-0xFFFF)
use super::register_file::Register;

#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Condition {
    NZ,
    Z,
    NC,
    C,
}

#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
/// R - from register 1
/// R_R - register to register transfer
/// R_D8 - immediate (PC) to register tranfer, n8 in the command sheet
/// MR_R - register to memory (destination is 1st register)
/// R_MR - memory to register
/// A8_R - register to immediate 8-bit (PC)
/// A16_R, D16_R - register to immediate (address) 16-bit (correspond to PC, PC + 1)
/// R_HLI, HLI_R - HLI (HL Increment), after accessing the memory address pointed to by HL, the HL register is incremented by 1.
/// R_HLD, HLD_R - HLD (HL Decrement), after accessing the memory address pointed to by HL, the HL register is decremented by 1.
/// Special memory (I/O)?
pub enum AddressMode {
    IMP,
    R_D16,
    R_R,
    MR_R,
    R,
    R_D8,
    R_MR,
    R_HLI,
    R_HLD,
    HLI_R,
    HLD_R,
    R_A8,
    A8_R,
    HL_SPR,
    D16,
    D8,
    D16_R,
    MR_D8,
    MR,
    A16_R,
    R_A16,
    RST,
}

#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum InstructionType {
    NONE,
    NOP,
    LD,
    INC,
    DEC,
    RLCA,
    ADD,
    RRCA,
    STOP,
    RLA,
    JR,
    RRA,
    DAA,
    CPL,
    SCF,
    CCF,
    HALT,
    ADC,
    SUB,
    SBC,
    AND,
    XOR,
    OR,
    CP,
    POP,
    JP,
    PUSH,
    RET,
    CB,
    CALL,
    RETI,
    LDH,
    JPHL,
    DI,
    EI,
    RST,
    ERR,
    //CB instructions...
    RLC,
    RRC,
    RL,
    RR,
    SLA,
    SRA,
    SWAP,
    SRL,
    BIT,
    RES,
    SET,
}

#[derive(Copy, Clone, Debug)]
pub struct Instruction {
    pub itype: InstructionType,
    pub mode: AddressMode,
    pub reg1: Option<Register>,
    pub reg2: Option<Register>,
    pub cond: Option<Condition>,
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            itype: InstructionType::NONE,
            mode: AddressMode::IMP,
            reg1: None,
            reg2: None,
            cond: None,
        }
    }
}

impl Instruction {
    fn get_register_for_prefixed(opcode: u8) -> Register {
        let reg_bits = opcode & 0b111; // equivalent to opcode % 8
        match reg_bits {
            0 => Register::B,
            1 => Register::C,
            2 => Register::D,
            3 => Register::E,
            4 => Register::H,
            5 => Register::L,
            6 => Register::HL,
            7 => Register::A,
            _ => panic!("Invalid register specifier {}", reg_bits),
        }
    }

    pub fn fmt_with_data(&self, data: u16) -> String {
        match self.mode {
            AddressMode::IMP => format!("{:?}", self.itype),
            AddressMode::D8 => format!("{:?} ${:02X}", self.itype, data),
            AddressMode::D16 => format!("{:?} ${:04X}", self.itype, data),
            AddressMode::R => format!("{:?} {:?}", self.itype, self.reg1.unwrap()),
            AddressMode::R_R => format!(
                "{:?} {:?}, {:?}",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::R_A8 | AddressMode::R_D8 => {
                format!("{:?} {:?}, ${:02X}", self.itype, self.reg1.unwrap(), data)
            }
            AddressMode::R_A16 | AddressMode::R_D16 => {
                format!("{:?} {:?}, ${:04X}", self.itype, self.reg1.unwrap(), data)
            }
            AddressMode::A8_R => {
                format!("{:?} ${:02X}, {:?}", self.itype, data, self.reg2.unwrap())
            }
            AddressMode::A16_R | AddressMode::D16_R => {
                format!("{:?} (${:04X}), {:?}", self.itype, data, self.reg2.unwrap())
            }
            AddressMode::MR => format!("{:?} ({:?})", self.itype, self.reg1.unwrap()),
            AddressMode::MR_R => format!(
                "{:?} ({:?}), {:?}",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::R_MR => format!(
                "{:?} {:?}, ({:?})",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::MR_D8 => {
                format!("{:?} ({:?}), ${:02X}", self.itype, self.reg1.unwrap(), data)
            }
            AddressMode::R_HLI => format!(
                "{:?} {:?}, ({:?}+)",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::HLI_R => format!(
                "{:?} ({:?}+), {:?}",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::R_HLD => format!(
                "{:?} {:?}, ({:?}-)",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::HLD_R => format!(
                "{:?} ({:?}-), {:?}",
                self.itype,
                self.reg1.unwrap(),
                self.reg2.unwrap()
            ),
            AddressMode::HL_SPR => format!(
                "{:?} {:?}, SP+{}",
                self.itype,
                self.reg1.unwrap(),
                data & 0xFF
            ),
            AddressMode::RST => format!("{:?} ${:02X}", self.itype, data),
        }
    }

    pub fn from_opcode_prefixed(opcode: u8) -> Self {
        let reg1 = Instruction::get_register_for_prefixed(opcode);
        let mode = if reg1 == Register::HL {
            AddressMode::MR
        } else {
            AddressMode::R
        };
        let itype_code = (opcode & 0xF0) >> 4;
        let itype = match itype_code {
            0 => {
                if opcode & 0x0F < 8 {
                    InstructionType::RLC
                } else {
                    InstructionType::RRC
                }
            }
            1 => {
                if opcode & 0x0F < 8 {
                    InstructionType::RL
                } else {
                    InstructionType::RR
                }
            }
            2 => {
                if opcode & 0x0F < 8 {
                    InstructionType::SLA
                } else {
                    InstructionType::SRA
                }
            }
            3 => {
                if opcode & 0x0F < 8 {
                    InstructionType::SWAP
                } else {
                    InstructionType::SRL
                }
            }
            4..=7 => InstructionType::BIT,
            8..=0xB => InstructionType::RES,
            0xC..=0xF => InstructionType::SET,
            _ => panic!("Invalid prefixed intruction code {:01X}", itype_code),
        };

        Instruction {
            itype,
            mode,
            reg1: Some(reg1),
            reg2: None,
            cond: None,
        }
    }

    pub fn from_opcode(opcode: u8) -> Self {
        match opcode {
            0x00 => Instruction {
                itype: InstructionType::NOP,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x01 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D16,
                reg1: Some(Register::BC),
                reg2: None,
                cond: None,
            },
            0x02 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::BC),
                reg2: Some(Register::A),
                cond: None,
            },
            0x03 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::BC),
                reg2: None,
                cond: None,
            },
            0x04 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::B),
                reg2: None,
                cond: None,
            },
            0x05 => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::B),
                reg2: None,
                cond: None,
            },
            0x06 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::B),
                reg2: None,
                cond: None,
            },
            0x07 => Instruction {
                itype: InstructionType::RLCA,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x08 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::A16_R,
                reg1: None,
                reg2: Some(Register::SP),
                cond: None,
            },
            0x09 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::BC),
                cond: None,
            },
            0x0A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::BC),
                cond: None,
            },
            0x0B => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::BC),
                reg2: None,
                cond: None,
            },
            0x0C => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::C),
                reg2: None,
                cond: None,
            },
            0x0D => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::C),
                reg2: None,
                cond: None,
            },
            0x0E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::C),
                reg2: None,
                cond: None,
            },
            0x0F => Instruction {
                itype: InstructionType::RRCA,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x10 => Instruction {
                itype: InstructionType::STOP,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x11 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D16,
                reg1: Some(Register::DE),
                reg2: None,
                cond: None,
            },
            0x12 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::DE),
                reg2: Some(Register::A),
                cond: None,
            },
            0x13 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::DE),
                reg2: None,
                cond: None,
            },
            0x14 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::D),
                reg2: None,
                cond: None,
            },
            0x15 => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::D),
                reg2: None,
                cond: None,
            },
            0x16 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::D),
                reg2: None,
                cond: None,
            },
            0x17 => Instruction {
                itype: InstructionType::RLA,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x18 => Instruction {
                itype: InstructionType::JR,
                mode: AddressMode::D8,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x19 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::DE),
                cond: None,
            },
            0x1A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::DE),
                cond: None,
            },
            0x1B => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::DE),
                reg2: None,
                cond: None,
            },
            0x1C => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::E),
                reg2: None,
                cond: None,
            },
            0x1D => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::E),
                reg2: None,
                cond: None,
            },
            0x1E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::E),
                reg2: None,
                cond: None,
            },
            0x1F => Instruction {
                itype: InstructionType::RRA,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x20 => Instruction {
                itype: InstructionType::JR,
                mode: AddressMode::D8,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NZ),
            },
            0x21 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D16,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x22 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::HLI_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::A),
                cond: None,
            },
            0x23 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x24 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::H),
                reg2: None,
                cond: None,
            },
            0x25 => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::H),
                reg2: None,
                cond: None,
            },
            0x26 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::H),
                reg2: None,
                cond: None,
            },
            0x27 => Instruction {
                itype: InstructionType::DAA,
                mode: AddressMode::R,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0x28 => Instruction {
                itype: InstructionType::JR,
                mode: AddressMode::D8,
                reg1: None,
                reg2: None,
                cond: Some(Condition::Z),
            },
            0x29 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x2A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_HLI,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x2B => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x2C => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::L),
                reg2: None,
                cond: None,
            },
            0x2D => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::L),
                reg2: None,
                cond: None,
            },
            0x2E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::L),
                reg2: None,
                cond: None,
            },
            0x2F => Instruction {
                itype: InstructionType::CPL,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x30 => Instruction {
                itype: InstructionType::JR,
                mode: AddressMode::D8,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NC),
            },
            0x31 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D16,
                reg1: Some(Register::SP),
                reg2: None,
                cond: None,
            },
            0x32 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::HLD_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::A),
                cond: None,
            },
            0x33 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::SP),
                reg2: None,
                cond: None,
            },
            0x34 => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::MR,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x35 => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::MR,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x36 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_D8,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0x37 => Instruction {
                itype: InstructionType::SCF,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x38 => Instruction {
                itype: InstructionType::JR,
                mode: AddressMode::D8,
                reg1: None,
                reg2: None,
                cond: Some(Condition::C),
            },
            0x39 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::SP),
                cond: None,
            },
            0x3A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_HLD,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x3B => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::SP),
                reg2: None,
                cond: None,
            },
            0x3C => Instruction {
                itype: InstructionType::INC,
                mode: AddressMode::R,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0x3D => Instruction {
                itype: InstructionType::DEC,
                mode: AddressMode::R,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0x3E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0x3F => Instruction {
                itype: InstructionType::CCF,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x40 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::B),
                cond: None,
            },
            0x41 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::C),
                cond: None,
            },
            0x42 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::D),
                cond: None,
            },
            0x43 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::E),
                cond: None,
            },
            0x44 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::H),
                cond: None,
            },
            0x45 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::L),
                cond: None,
            },
            0x46 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::B),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x47 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::B),
                reg2: Some(Register::A),
                cond: None,
            },
            0x48 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::B),
                cond: None,
            },
            0x49 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::C),
                cond: None,
            },
            0x4A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::D),
                cond: None,
            },
            0x4B => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::E),
                cond: None,
            },
            0x4C => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::H),
                cond: None,
            },
            0x4D => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::L),
                cond: None,
            },
            0x4E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::C),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x4F => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::C),
                reg2: Some(Register::A),
                cond: None,
            },
            0x50 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::B),
                cond: None,
            },
            0x51 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::C),
                cond: None,
            },
            0x52 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::D),
                cond: None,
            },
            0x53 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::E),
                cond: None,
            },
            0x54 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::H),
                cond: None,
            },
            0x55 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::L),
                cond: None,
            },
            0x56 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::D),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x57 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::D),
                reg2: Some(Register::A),
                cond: None,
            },
            0x58 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::B),
                cond: None,
            },
            0x59 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::C),
                cond: None,
            },
            0x5A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::D),
                cond: None,
            },
            0x5B => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::E),
                cond: None,
            },
            0x5C => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::H),
                cond: None,
            },
            0x5D => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::L),
                cond: None,
            },
            0x5E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::E),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x5F => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::E),
                reg2: Some(Register::A),
                cond: None,
            },
            0x60 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::B),
                cond: None,
            },
            0x61 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::C),
                cond: None,
            },
            0x62 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::D),
                cond: None,
            },
            0x63 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::E),
                cond: None,
            },
            0x64 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::H),
                cond: None,
            },
            0x65 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::L),
                cond: None,
            },
            0x66 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::H),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x67 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::H),
                reg2: Some(Register::A),
                cond: None,
            },
            0x68 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::B),
                cond: None,
            },
            0x69 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::C),
                cond: None,
            },
            0x6A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::D),
                cond: None,
            },
            0x6B => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::E),
                cond: None,
            },
            0x6C => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::H),
                cond: None,
            },
            0x6D => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::L),
                cond: None,
            },
            0x6E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::L),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x6F => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::L),
                reg2: Some(Register::A),
                cond: None,
            },
            0x70 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::B),
                cond: None,
            },
            0x71 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::C),
                cond: None,
            },
            0x72 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::D),
                cond: None,
            },
            0x73 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::E),
                cond: None,
            },
            0x74 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::H),
                cond: None,
            },
            0x75 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::L),
                cond: None,
            },
            0x76 => Instruction {
                itype: InstructionType::HALT,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0x77 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::MR_R,
                reg1: Some(Register::HL),
                reg2: Some(Register::A),
                cond: None,
            },
            0x78 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },
            0x79 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },
            0x7A => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },
            0x7B => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },
            0x7C => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },
            0x7D => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },
            0x7E => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x7F => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },
            0x80 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0x81 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0x82 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0x83 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0x84 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0x85 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },
            0x86 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },
            0x87 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },
            0x88 => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0x89 => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0x8A => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0x8B => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0x8C => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0x8D => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0x8E => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0x8F => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },
            0x90 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0x91 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0x92 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0x93 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0x94 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0x95 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0x96 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0x97 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },

            0x98 => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0x99 => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0x9A => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0x9B => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0x9C => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0x9D => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0x9E => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0x9F => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },

            0xA0 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0xA1 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0xA2 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0xA3 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0xA4 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0xA5 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0xA6 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0xA7 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },

            0xA8 => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0xA9 => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0xAA => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0xAB => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0xAC => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0xAD => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0xAE => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0xAF => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },

            0xB0 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0xB1 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0xB2 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0xB3 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0xB4 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0xB5 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0xB6 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0xB7 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },

            0xB8 => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::B),
                cond: None,
            },

            0xB9 => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },

            0xBA => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::D),
                cond: None,
            },

            0xBB => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::E),
                cond: None,
            },

            0xBC => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::H),
                cond: None,
            },

            0xBD => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::L),
                cond: None,
            },

            0xBE => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::HL),
                cond: None,
            },

            0xBF => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_R,
                reg1: Some(Register::A),
                reg2: Some(Register::A),
                cond: None,
            },
            0xC0 => Instruction {
                itype: InstructionType::RET,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NZ),
            },
            0xC1 => Instruction {
                itype: InstructionType::POP,
                mode: AddressMode::R,
                reg1: Some(Register::BC),
                reg2: None,
                cond: None,
            },
            0xC2 => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NZ),
            },
            0xC3 => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xC4 => Instruction {
                itype: InstructionType::CALL,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NZ),
            },
            0xC5 => Instruction {
                itype: InstructionType::PUSH,
                mode: AddressMode::R,
                reg1: Some(Register::BC),
                reg2: None,
                cond: None,
            },
            0xC6 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xC7 => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xC8 => Instruction {
                itype: InstructionType::RET,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: Some(Condition::Z),
            },
            0xC9 => Instruction {
                itype: InstructionType::RET,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xCA => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::Z),
            },
            0xCB => panic!("CB prefix instructions are not ready yet!"),
            0xCC => Instruction {
                itype: InstructionType::CALL,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::Z),
            },
            0xCD => Instruction {
                itype: InstructionType::CALL,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xCE => Instruction {
                itype: InstructionType::ADC,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xCF => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xD0 => Instruction {
                itype: InstructionType::RET,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NC),
            },
            0xD1 => Instruction {
                itype: InstructionType::POP,
                mode: AddressMode::R,
                reg1: Some(Register::DE),
                reg2: None,
                cond: None,
            },
            0xD2 => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NC),
            },
            0xD3 => panic!("Illegal opcode 0x{opcode:X}"),
            0xD4 => Instruction {
                itype: InstructionType::CALL,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::NC),
            },
            0xD5 => Instruction {
                itype: InstructionType::PUSH,
                mode: AddressMode::R,
                reg1: Some(Register::DE),
                reg2: None,
                cond: None,
            },
            0xD6 => Instruction {
                itype: InstructionType::SUB,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xD7 => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xD8 => Instruction {
                itype: InstructionType::RET,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: Some(Condition::C),
            },
            0xD9 => Instruction {
                itype: InstructionType::RETI,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xDA => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::C),
            },
            0xDB => panic!("Illegal opcode 0x{opcode:X}"),
            0xDC => Instruction {
                itype: InstructionType::CALL,
                mode: AddressMode::D16,
                reg1: None,
                reg2: None,
                cond: Some(Condition::C),
            },
            0xDD => panic!("Illegal opcode 0x{opcode:X}"),
            0xDE => Instruction {
                itype: InstructionType::SBC,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xDF => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xE0 => Instruction {
                itype: InstructionType::LDH,
                mode: AddressMode::A8_R,
                reg1: None,
                reg2: Some(Register::A),
                cond: None,
            },
            0xE1 => Instruction {
                itype: InstructionType::POP,
                mode: AddressMode::R,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0xE2 => Instruction {
                itype: InstructionType::LDH,
                mode: AddressMode::MR_R,
                reg1: Some(Register::C),
                reg2: Some(Register::A),
                cond: None,
            },
            0xE3 => panic!("Illegal opcode 0x{opcode:X}"),
            0xE4 => panic!("Illegal opcode 0x{opcode:X}"),
            0xE5 => Instruction {
                itype: InstructionType::PUSH,
                mode: AddressMode::R,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0xE6 => Instruction {
                itype: InstructionType::AND,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xE7 => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xE8 => Instruction {
                itype: InstructionType::ADD,
                mode: AddressMode::R_D8,
                reg1: Some(Register::SP),
                reg2: None,
                cond: None,
            },
            0xE9 => Instruction {
                itype: InstructionType::JP,
                mode: AddressMode::R,
                reg1: Some(Register::HL),
                reg2: None,
                cond: None,
            },
            0xEA => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::A16_R,
                reg1: None,
                reg2: Some(Register::A),
                cond: None,
            },
            0xEB => panic!("Illegal opcode 0x{opcode:X}"),
            0xEC => panic!("Illegal opcode 0x{opcode:X}"),
            0xED => panic!("Illegal opcode 0x{opcode:X}"),
            0xEE => Instruction {
                itype: InstructionType::XOR,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xEF => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xF0 => Instruction {
                itype: InstructionType::LDH,
                mode: AddressMode::R_A8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xF1 => Instruction {
                itype: InstructionType::POP,
                mode: AddressMode::R,
                reg1: Some(Register::AF),
                reg2: None,
                cond: None,
            },
            0xF2 => Instruction {
                itype: InstructionType::LDH,
                mode: AddressMode::R_MR,
                reg1: Some(Register::A),
                reg2: Some(Register::C),
                cond: None,
            },
            0xF3 => Instruction {
                itype: InstructionType::DI,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xF4 => panic!("Illegal opcode 0x{opcode:X}"),
            0xF5 => Instruction {
                itype: InstructionType::PUSH,
                mode: AddressMode::R,
                reg1: Some(Register::AF),
                reg2: None,
                cond: None,
            },
            0xF6 => Instruction {
                itype: InstructionType::OR,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xF7 => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xF8 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::HL_SPR,
                reg1: Some(Register::HL),
                reg2: Some(Register::SP),
                cond: None,
            },
            0xF9 => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_R,
                reg1: Some(Register::SP),
                reg2: Some(Register::HL),
                cond: None,
            },
            0xFA => Instruction {
                itype: InstructionType::LD,
                mode: AddressMode::R_A16,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xFB => Instruction {
                itype: InstructionType::EI,
                mode: AddressMode::IMP,
                reg1: None,
                reg2: None,
                cond: None,
            },
            0xFC => panic!("Illegal opcode 0x{opcode:X}"),
            0xFD => panic!("Illegal opcode 0x{opcode:X}"),
            0xFE => Instruction {
                itype: InstructionType::CP,
                mode: AddressMode::R_D8,
                reg1: Some(Register::A),
                reg2: None,
                cond: None,
            },
            0xFF => Instruction {
                itype: InstructionType::RST,
                mode: AddressMode::RST,
                reg1: None,
                reg2: None,
                cond: None,
            },
        }
    }
}
