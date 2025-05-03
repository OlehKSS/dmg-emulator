mod instructions;
mod register_file;

use crate::bus::MemoryBus;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use instructions::*;
use register_file::{Register, RegisterFile};

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

    pub fn step(&mut self) -> bool {
        if !self.halted {
            self.fetch_instruction();
            self.fetch_data();
            println!("Executing {:?}\n{}", self.instruction.itype, self.registers);
            self.execute();
            true
        } else {
            self.ctx.borrow_mut().tick_cycle();
            // if (ctx.int_flags) {
            //     ctx.halted = false;
            // }
            false
        }
    }

    fn fetch_instruction(&mut self) {
        self.cur_opcode = self.bus.borrow().read(self.registers.pc);
        self.registers.pc += 1;
        self.instruction = Instruction::from_opcode(self.cur_opcode);
        self.ctx.borrow_mut().tick_cycle();
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
            AddressMode::R_R => {
                self.fetched_data = self.registers.read8(self.instruction.reg2.unwrap()) as u16
            }
            AddressMode::R_D8 => {
                self.fetched_data = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1;
            }
            AddressMode::R_D16 | AddressMode::D16 => {
                let lo = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                let hi = self.bus.borrow().read(self.registers.pc + 1) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.fetched_data = lo | (hi << 8); // Little or Big Endian?
            }
            AddressMode::R_HLI => {
                let reg2 = self.instruction.reg2.unwrap();
                assert!(reg2 == Register::HL);
                let address = self.registers.read16(reg2);
                self.fetched_data = self.bus.borrow().read(address) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers
                    .write16(Register::HL, address.wrapping_add(1));
            }
            AddressMode::R_HLD => {
                let reg2 = self.instruction.reg2.unwrap();
                assert!(reg2 == Register::HL);
                let address = self.registers.read16(reg2);
                self.fetched_data = self.bus.borrow().read(address) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers
                    .write16(Register::HL, address.wrapping_sub(1));
            }
            AddressMode::HLI_R => {
                let reg1 = self.instruction.reg1.unwrap();
                assert!(reg1 == Register::HL);
                let address = self.registers.read16(reg1);
                self.mem_dest = address;
                self.fetched_data = self.registers.read8(self.instruction.reg2.unwrap()) as u16;
                self.dest_is_mem = true;
                self.registers
                    .write16(Register::HL, address.wrapping_add(1));
            }
            AddressMode::HLD_R => {
                let reg1 = self.instruction.reg1.unwrap();
                assert!(reg1 == Register::HL);
                let address = self.registers.read16(reg1);
                self.mem_dest = address;
                self.fetched_data = self.registers.read8(self.instruction.reg2.unwrap()) as u16;
                self.dest_is_mem = true;
                self.registers
                    .write16(Register::HL, address.wrapping_sub(1));
            }
            AddressMode::HL_SPR => {
                // TODO: Is it supposed to be stack ptr?
                self.fetched_data = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1;
            }
            AddressMode::MR_R => {
                let reg1 = self.instruction.reg1.unwrap();
                self.fetched_data = self.registers.read8(self.instruction.reg2.unwrap()) as u16;
                self.mem_dest = if reg1 == Register::C {
                    (self.registers.read8(reg1) as u16) | 0xFF00
                } else {
                    self.registers.read16(reg1)
                };

                self.dest_is_mem = true;
            }
            AddressMode::R_MR => {
                let reg2 = self.instruction.reg2.unwrap();
                let address = if reg2 == Register::C {
                    (self.registers.read8(reg2) as u16) | 0xFF00
                } else {
                    self.registers.read16(reg2)
                };
                // Ticks happen because of reads, could we combine it?
                self.fetched_data = self.bus.borrow().read(address) as u16;
                self.ctx.borrow_mut().tick_cycle();
            }
            AddressMode::R_A8 | AddressMode::D8 => {
                // Stubs or final implementation?
                self.fetched_data = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1;
            }
            AddressMode::A8_R => {
                self.dest_is_mem = true;
                self.mem_dest = (self.bus.borrow().read(self.registers.pc) as u16) | 0xFF00;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1; // Should probably be wrapping add everywhere
            }
            AddressMode::MR => {
                let reg1 = self.registers.read16(self.instruction.reg1.unwrap());
                self.mem_dest = reg1;
                self.dest_is_mem = true;
                self.fetched_data = self.bus.borrow().read(reg1) as u16;
                self.ctx.borrow_mut().tick_cycle();
            }
            AddressMode::MR_D8 => {
                self.fetched_data = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.registers.pc += 1;
                self.mem_dest = self.registers.read16(self.instruction.reg1.unwrap());
                self.dest_is_mem = true;
            }
            AddressMode::A16_R | AddressMode::D16_R => {
                let lo = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                let hi = self.bus.borrow().read(self.registers.pc + 1) as u16;
                self.ctx.borrow_mut().tick_cycle();
                self.mem_dest = lo | (hi << 8);
                self.dest_is_mem = true;
                self.registers.pc += 2;
                self.fetched_data = self.registers.read8(self.instruction.reg2.unwrap()) as u16;
            }
            AddressMode::R_A16 => {
                let lo = self.bus.borrow().read(self.registers.pc) as u16;
                self.ctx.borrow_mut().tick_cycle();
                let hi = self.bus.borrow().read(self.registers.pc + 1) as u16;
                self.ctx.borrow_mut().tick_cycle();

                let address = lo | hi << 8;

                self.registers.pc += 2;
                self.fetched_data = self.bus.borrow().read(address) as u16;
                self.ctx.borrow_mut().tick_cycle();
            }
            _ => panic!("Unknown addressing mode {}", self.instruction.mode as u8),
        }
    }

    fn execute(&mut self) {
        match self.instruction.itype {
            InstructionType::NONE => {
                // TODO: Should we remove it?
                panic!("Invalid instruction NONE");
            }
            InstructionType::NOP => {
                // Nothing to do
            }
            InstructionType::JP => {
                self.jump();
            }
            InstructionType::LD => {
                self.load();
            }
            InstructionType::LDH => {
                self.load_high();
            }
            InstructionType::XOR => {
                self.xor();
            }
            _ => panic!("Instruction {:?} not implemented.", self.instruction.itype),
        }
    }

    fn check_flags(&self) -> bool {
        if let Some(cond) = self.instruction.cond {
            return match cond {
                Condition::C => self.registers.cf(),
                Condition::NC => !self.registers.cf(),
                Condition::NZ => !self.registers.zf(),
                Condition::Z => self.registers.zf(),
            };
        }

        true
    }

    fn jump(&mut self) {
        if self.check_flags() {
            self.registers.pc = self.fetched_data;
            self.ctx.borrow_mut().tick_cycle();
        }
    }

    fn load(&mut self) {
        if self.dest_is_mem {
            let reg2 = self.instruction.reg2.unwrap();
            if reg2.is_16bit() {
                self.ctx.borrow_mut().tick_cycle();
                self.bus
                    .borrow_mut()
                    .write16(self.mem_dest, self.fetched_data);
            } else {
                self.bus
                    .borrow_mut()
                    .write(self.mem_dest, self.fetched_data as u8);
            }

            self.ctx.borrow_mut().tick_cycle();
            return;
        }

        if self.instruction.mode == AddressMode::HL_SPR {
            todo!("Implement LD for AddressMode::HL_SPR");
        }

        let reg1 = self.instruction.reg1.unwrap();
        self.registers.write8(reg1, self.fetched_data as u8);
    }

    fn load_high(&mut self) {
        if self.dest_is_mem {
            self.bus
                .borrow_mut()
                .write(self.mem_dest, self.fetched_data as u8);
        } else {
            assert!(self.instruction.reg1.unwrap() == Register::A);
            let address = 0xFF00 | self.fetched_data;
            let data = self.bus.borrow().read(address);
            self.registers.write8(Register::A, data);
        }

        self.ctx.borrow_mut().tick_cycle();
    }

    /// XOR s
    ///
    /// Flags: Z N H C
    ///        * 0 0 0
    fn xor(&mut self) {
        let value = self.registers.read8(Register::A) ^ ((self.fetched_data & 0x00FF) as u8);
        self.registers.write8(Register::A, value);
        self.registers.set_zf(value == 0);
        self.registers.set_cf(false);
        self.registers.set_hf(false);
        self.registers.set_nf(false);
    }
}

impl fmt::Display for CPU<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CPU register file:\n{}", self.registers)
    }
}
