mod instructions;
mod register_file;

use crate::bus::MemoryBus;
use std::cell::RefCell;
use std::rc::Rc;

use instructions::*;
use register_file::RegisterFile;

// #[derive(Debug)]
#[allow(dead_code)]
pub struct CPU<'a> {
    registers: RegisterFile,
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
        CPU {
            registers: RegisterFile::new(),
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

    fn fetch_instruction(&mut self) {
        self.cur_opcode = self.bus.borrow().read(self.registers.pc);
        self.registers.pc += 1;
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
                self.fetched_data = self.registers.read8(self.instruction.reg1.unwrap()) as u16
            }
            AddressMode::R_D8 => {
                self.fetched_data = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1;
            }
            AddressMode::D16 => {
                let lo = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                let hi = self.bus.borrow().read(self.registers.pc + 1) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.fetched_data = lo | (hi << 8); // Little or Big Endian?
            }
            _ => panic!("Unknown addressing mode {}", self.instruction.mode as u8),
        }
    }
}
