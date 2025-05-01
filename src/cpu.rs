use crate::bus::MemoryBus;
use std::cell::RefCell;
use std::rc::Rc;

use super::emu::Emulator;
use super::instructions::Register;
use super::instructions::*;

macro_rules! bit_set {
    ($a:expr, $n:expr, $on:expr) => {
        if $on {
            $a |= 1 << $n;
        } else {
            $a &= !(1 << $n);
        }
    };
}

// #[derive(Debug)]
#[allow(dead_code)]
pub struct CPU<'a> {
    // registers: Registers
    registers: [u8; 8],
    pc: u16,
    sp: u16,

    // Current fetch
    fetched_data: u16,
    mem_dest: u16,
    dest_is_mem: bool,
    cur_opcode: u8,
    instruction: Instruction,

    halted: bool,
    stepping: bool,
    int_master_enabled: bool,

    bus: RefCell<&'a mut MemoryBus>,
    ctx: Rc<RefCell<dyn CpuContext>>,
}

pub trait CpuContext {
    fn tick_cycle(&mut self);
}

impl<'a> CPU<'a> {
    pub fn new(bus: &'a mut MemoryBus, ctx: Rc<RefCell<dyn CpuContext>>) -> Self {
        // CPU { registers: Registers::default() }
        // Initial register values should be set according to DMG spec
        let mut registers = [0; 8];
        registers[Register::A as usize] = 0x01;

        CPU {
            registers,
            pc: 0x100,
            sp: 0,
            fetched_data: 0,
            mem_dest: 0,
            dest_is_mem: false,
            cur_opcode: 0,
            instruction: Instruction::default(),
            halted: false,
            stepping: true,
            int_master_enabled: false,
            bus: RefCell::new(bus),
            ctx,
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
        self.cur_opcode = self.bus.borrow().read(self.pc);
        self.pc += 1;
        self.instruction = Instruction::from_opcode(self.cur_opcode);
    }

    fn fetch_data(&mut self) {
        self.mem_dest = 0;
        self.dest_is_mem = false;

        if self.instruction.itype == InstructionType::NONE {
            return;
        }

        match self.instruction.mode {
            AddressMode::IMP => (),
            AddressMode::R => {
                self.fetched_data = self.read_register(self.instruction.reg1.unwrap())
            }
            AddressMode::R_D8 => {
                self.fetched_data = self.bus.borrow().read(self.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.pc += 1;
            }
            AddressMode::D16 => {
                let lo = self.bus.borrow().read(self.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                let hi = self.bus.borrow().read(self.pc + 1) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.fetched_data = lo | (hi << 8); // Little or Big Endian?
            }
            _ => panic!("Unknown addressing mode {}", self.instruction.mode as u8),
        }
    }

    /// Set register if the argument value above 0, unset if it equals to 0, and ignore if set to -1.
    ///
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
    fn set_flags(&mut self, z: i8, n: i8, h: i8, c: i8) {
        if z != -1 {
            bit_set!(self.registers[Register::F as usize], 7, z > 0);
        }
        if n != -1 {
            bit_set!(self.registers[Register::F as usize], 6, n > 0);
        }
        if h != -1 {
            bit_set!(self.registers[Register::F as usize], 5, h > 0);
        }
        if c != -1 {
            bit_set!(self.registers[Register::F as usize], 4, c > 0);
        }
    }
}
