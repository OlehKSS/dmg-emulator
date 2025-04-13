use super::bus::MemoryBus;
use super::instructions::Instruction;
use super::instructions::Register;

// #[derive(Debug)]
pub struct CPU<'a> {
    // registers: Registers
    registers: [u8; 8],
    pc: u16,
    sp: u16,
    cur_opcode: u8,
    instruction: Instruction,
    bus: &'a mut MemoryBus,
}

impl<'a> CPU<'a> {
    pub fn new(bus: &'a mut MemoryBus) -> Self {
        // CPU { registers: Registers::default() }
        let mut registers = [0; 8];
        registers[Register::A as usize] = 0x01;

        CPU {
            registers,
            pc: 0x100,
            sp: 0,
            cur_opcode: 0,
            instruction: Instruction::default(),
            bus,
        }
    }

    pub fn step(&self) -> bool {
        false
    }

    pub fn read_register(&self, reg: Register) -> u16 {
        match reg {
            Register::A
            | Register::F
            | Register::B
            | Register::C
            | Register::D
            | Register::E
            | Register::H
            | Register::L => self.registers[reg as usize] as u16,
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
        }
    }

    fn fetch_instruction(&mut self) {
        self.cur_opcode = self.bus.read(self.pc);
        self.pc += 1;
        self.instruction = Instruction::from_opcode(self.cur_opcode);
    }
}
