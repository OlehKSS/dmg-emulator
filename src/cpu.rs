mod instructions;
mod interrupt;
mod register_file;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use instructions::*;
use interrupt::{InterruptFlag, get_hadler_address};
use register_file::{Register, RegisterFile};

// #[derive(Debug)]
#[allow(dead_code)]
pub struct CPU {
    registers: RegisterFile,
    // Current fetch
    fetched_data: u16,
    mem_dest: u16,
    dest_is_mem: bool,
    cur_opcode: u8,
    instruction: Instruction,

    halted: bool,
    stepping: bool,
    ime: bool,
    ime_scheduled: bool,

    ctx: Rc<RefCell<dyn CpuContext>>,
}

pub trait CpuContext {
    fn tick_cycle(&mut self);
    fn read_cycle(&mut self, address: u16) -> u8;
    fn write_cycle(&mut self, address: u16, value: u8);
}

impl CPU {
    pub fn new(ctx: Rc<RefCell<dyn CpuContext>>) -> Self {
        CPU {
            registers: RegisterFile::new(),
            fetched_data: 0,
            mem_dest: 0,
            dest_is_mem: false,
            cur_opcode: 0,
            instruction: Instruction::default(),
            halted: false,
            stepping: true,
            ime: false,
            ime_scheduled: false,
            ctx,
        }
    }

    pub fn step(&mut self) -> bool {
        if !self.halted {
            self.fetch_instruction();
            println!(
                "Executing {:?} 0x{:02X} \n{}",
                self.instruction.itype, self.cur_opcode, self.registers
            );
            self.fetch_data();
            self.execute();
            // status = true;
        } else {
            const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;
            const INTERRUPT_FLAG_REGISTER: u16 = 0xFF0F;
            let ier = self.ctx.borrow_mut().read_cycle(INTERRUPT_ENABLE_REGISTER);
            let ifr = self.ctx.borrow_mut().read_cycle(INTERRUPT_FLAG_REGISTER);

            if ifr & ier != 0 {
                // Resume if an interrupt is requested
                self.halted = false;
            }
            self.ctx.borrow_mut().tick_cycle();
            return false;
        }

        if self.ime {
            self.handle_interrupts();
            self.ime_scheduled = false;
        }

        if self.ime_scheduled {
            self.ime = true;
        }

        true
    }

    fn fetch_instruction(&mut self) {
        self.cur_opcode = self.ctx.borrow_mut().read_cycle(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
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
                let reg = self.instruction.reg1.unwrap();

                if reg.is_16bit() {
                    self.fetched_data = self.registers.read16(reg);
                } else {
                    self.fetched_data = self.registers.read8(reg) as u16;
                }
            }
            AddressMode::R_R => {
                let reg = self.instruction.reg2.unwrap();

                if reg.is_16bit() {
                    self.fetched_data = self.registers.read16(reg);
                } else {
                    self.fetched_data = self.registers.read8(reg) as u16;
                }
            }
            AddressMode::R_D8 => {
                self.fetched_data = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                self.registers.pc = self.registers.pc.wrapping_add(1);
            }
            AddressMode::R_D16 | AddressMode::D16 => {
                let lo = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                let hi = self
                    .ctx
                    .borrow_mut()
                    .read_cycle(self.registers.pc.wrapping_add(1)) as u16;
                self.fetched_data = lo | (hi << 8);
                self.registers.pc = self.registers.pc.wrapping_add(2);
            }
            AddressMode::R_HLI => {
                let reg2 = self.instruction.reg2.unwrap();
                assert!(reg2 == Register::HL);
                let address = self.registers.read16(reg2);
                self.fetched_data = self.ctx.borrow_mut().read_cycle(address) as u16;
                self.registers
                    .write16(Register::HL, address.wrapping_add(1));
            }
            AddressMode::R_HLD => {
                let reg2 = self.instruction.reg2.unwrap();
                assert!(reg2 == Register::HL);
                let address = self.registers.read16(reg2);
                self.fetched_data = self.ctx.borrow_mut().read_cycle(address) as u16;
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
                self.fetched_data = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                self.registers.pc = self.registers.pc.wrapping_add(1);
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
                self.fetched_data = self.ctx.borrow_mut().read_cycle(address) as u16;
            }
            AddressMode::R_A8 | AddressMode::D8 => {
                // Stubs or final implementation?
                self.fetched_data = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                self.registers.pc = self.registers.pc.wrapping_add(1);
            }
            AddressMode::A8_R => {
                self.dest_is_mem = true;
                self.mem_dest =
                    (self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16) | 0xFF00;
                self.registers.pc = self.registers.pc.wrapping_add(1); // Should probably be wrapping add everywhere
            }
            AddressMode::MR => {
                let reg1 = self.registers.read16(self.instruction.reg1.unwrap());
                self.mem_dest = reg1;
                self.dest_is_mem = true;
                self.fetched_data = self.ctx.borrow_mut().read_cycle(reg1) as u16;
            }
            AddressMode::MR_D8 => {
                self.fetched_data = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                self.registers.pc = self.registers.pc.wrapping_add(1);
                self.mem_dest = self.registers.read16(self.instruction.reg1.unwrap());
                self.dest_is_mem = true;
            }
            AddressMode::A16_R | AddressMode::D16_R => {
                let lo = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                let hi = self
                    .ctx
                    .borrow_mut()
                    .read_cycle(self.registers.pc.wrapping_add(1)) as u16;
                self.mem_dest = lo | (hi << 8);
                self.dest_is_mem = true;
                self.registers.pc = self.registers.pc.wrapping_add(2);

                let reg2 = self.instruction.reg2.unwrap();

                if reg2.is_16bit() {
                    self.fetched_data = self.registers.read16(reg2);
                } else {
                    self.fetched_data = self.registers.read8(reg2) as u16;
                }
            }
            AddressMode::R_A16 => {
                let lo = self.ctx.borrow_mut().read_cycle(self.registers.pc) as u16;
                let hi = self
                    .ctx
                    .borrow_mut()
                    .read_cycle(self.registers.pc.wrapping_add(1)) as u16;

                let address = lo | hi << 8;

                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.fetched_data = self.ctx.borrow_mut().read_cycle(address) as u16;
            }
            AddressMode::RST => {
                self.fetched_data = match self.cur_opcode {
                    0xC7 => 0x00,
                    0xCF => 0x08,
                    0xD7 => 0x10,
                    0xDF => 0x18,
                    0xE7 => 0x20,
                    0xEF => 0x28,
                    0xF7 => 0x30,
                    0xFF => 0x38,
                    _ => panic!("Invalid opcode for RST {}", self.cur_opcode),
                };
            }
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
            InstructionType::HALT => {
                self.halt();
            }
            InstructionType::DI => {
                self.disable_interrupts();
            }
            InstructionType::EI => {
                self.enable_interrupts();
            }
            InstructionType::DEC => {
                self.decrement();
            }
            InstructionType::INC => {
                self.increment();
            }
            InstructionType::JP => {
                self.jump();
            }
            InstructionType::JR => {
                self.jump_rel();
            }
            InstructionType::LD => {
                self.load();
            }
            InstructionType::LDH => {
                self.load_high();
            }
            InstructionType::CALL => {
                self.call();
            }
            InstructionType::RST => {
                self.rst();
            }
            InstructionType::RET => {
                self.ret();
            }
            InstructionType::RETI => {
                self.enable_interrupts();
                self.ret();
            }
            InstructionType::POP => {
                self.pop();
            }
            InstructionType::PUSH => {
                self.push();
            }
            InstructionType::CCF => {
                self.ccf();
            }
            InstructionType::SCF => {
                self.scf();
            }
            InstructionType::CPL => {
                self.cpl();
            }
            InstructionType::DAA => {
                self.daa();
            }
            InstructionType::ADC => {
                self.adc();
            }
            InstructionType::ADD => {
                self.add();
            }
            InstructionType::CP => {
                self.cp();
            }
            InstructionType::SBC => {
                self.sbc();
            }
            InstructionType::SUB => {
                self.sub();
            }
            InstructionType::AND => {
                self.and();
            }
            InstructionType::OR => {
                self.or();
            }
            InstructionType::XOR => {
                self.xor();
            }
            InstructionType::RLA => {
                self.rla();
            }
            InstructionType::RLCA => {
                self.rlca();
            }
            InstructionType::RRA => {
                self.rra();
            }
            InstructionType::RRCA => {
                self.rrca();
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

    fn halt(&mut self) {
        self.halted = true;
    }

    fn disable_interrupts(&mut self) {
        self.ime = false;
    }

    fn enable_interrupts(&mut self) {
        self.ime_scheduled = true;
    }

    fn handle_interrupts(&mut self) {
        const INTERRUPT_ENABLE_REGISTER: u16 = 0xFFFF;
        const INTERRUPT_FLAG_REGISTER: u16 = 0xFF0F;
        let ier = self.ctx.borrow_mut().read_cycle(INTERRUPT_ENABLE_REGISTER);
        let ifr = self.ctx.borrow_mut().read_cycle(INTERRUPT_FLAG_REGISTER);
        let interrupts = InterruptFlag::from_bits_truncate(ier & ifr); // Enabled and requested interrupts

        if interrupts.is_empty() {
            return;
        }

        self.ime = false;
        // Clear the interrupt flag
        self.ctx.borrow_mut().write_cycle(
            INTERRUPT_FLAG_REGISTER,
            ifr & !(interrupts.highest_priority().bits()),
        );

        self.push_value(self.registers.pc);
        self.registers.pc = get_hadler_address(interrupts);
        self.ctx.borrow_mut().tick_cycle(); // How many cycles?
    }

    /// DEC s
    ///
    /// Flags: Z N H C (8-bit)
    ///        * 1 * -
    fn decrement(&mut self) {
        let reg1 = self.instruction.reg1.unwrap();

        if reg1.is_16bit() && !self.dest_is_mem {
            // Does not change flags
            let value = self.fetched_data.wrapping_sub(1);
            self.registers.write16(reg1, value);
            return;
        }

        let value = self.fetched_data as u8;
        let result = value.wrapping_sub(1);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(true);
        self.registers.set_hf((value & 0x0F) == 0x00);

        if self.dest_is_mem {
            self.ctx.borrow_mut().write_cycle(self.mem_dest, result);
        } else {
            self.registers.write8(reg1, result);
        }
    }

    /// INC s
    ///
    /// Flags: Z N H C (8-bit)
    ///        * 0 * -
    fn increment(&mut self) {
        let reg1 = self.instruction.reg1.unwrap();

        if reg1.is_16bit() && !self.dest_is_mem {
            // Does not change flags
            let value = self.fetched_data.wrapping_add(1);
            self.registers.write16(reg1, value);
            return;
        }

        let value = self.fetched_data as u8;
        let result = value.wrapping_add(1);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf((value & 0x0F) == 0x0F);

        if self.dest_is_mem {
            self.ctx.borrow_mut().write_cycle(self.mem_dest, result);
        } else {
            self.registers.write8(reg1, result);
        }
    }

    fn jump(&mut self) {
        if self.check_flags() {
            self.registers.pc = self.fetched_data;
            self.ctx.borrow_mut().tick_cycle();
        }
    }

    fn jump_rel(&mut self) {
        if self.check_flags() {
            let e8 = self.fetched_data & 0xFF;
            self.registers.pc = self.registers.pc.wrapping_add(e8);
            self.ctx.borrow_mut().tick_cycle();
        }
    }

    fn load(&mut self) {
        if self.dest_is_mem {
            let reg2 = self.instruction.reg2.unwrap();
            if reg2.is_16bit() {
                self.ctx
                    .borrow_mut()
                    .write_cycle(self.mem_dest, self.fetched_data as u8); // lo
                self.ctx.borrow_mut().write_cycle(
                    self.mem_dest.wrapping_add(1),
                    (self.fetched_data >> 8) as u8,
                ); // hi
            } else {
                self.ctx
                    .borrow_mut()
                    .write_cycle(self.mem_dest, self.fetched_data as u8);
            }
            return;
        }

        if self.instruction.mode == AddressMode::HL_SPR {
            todo!("Implement LD for AddressMode::HL_SPR");
        }

        let reg1 = self.instruction.reg1.unwrap();

        if reg1.is_16bit() {
            self.registers.write16(reg1, self.fetched_data);
        } else {
            self.registers.write8(reg1, self.fetched_data as u8);
        }
    }

    fn load_high(&mut self) {
        if self.dest_is_mem {
            self.ctx
                .borrow_mut()
                .write_cycle(self.mem_dest, self.fetched_data as u8);
        } else {
            assert!(self.instruction.reg1.unwrap() == Register::A);
            let address = 0xFF00 | self.fetched_data;
            let data = self.ctx.borrow_mut().read_cycle(address);
            self.registers.write8(Register::A, data);
        }
    }

    fn call(&mut self) {
        if self.check_flags() {
            self.push_value(self.registers.pc);
            self.registers.pc = self.fetched_data;
        }
    }

    fn rst(&mut self) {
        self.push_value(self.registers.pc);
        self.registers.pc = self.fetched_data;
    }

    fn ret(&mut self) {
        if self.check_flags() {
            self.registers.pc = self.pop_value();
            self.ctx.borrow_mut().tick_cycle();
        }
    }

    /// POP rr
    ///
    /// Flags: Z N H C
    ///        - - - -
    /// Note! POP AF affects all flags
    fn pop(&mut self) {
        let value = self.pop_value();
        self.registers
            .write16(self.instruction.reg1.unwrap(), value);
    }

    fn pop_value(&mut self) -> u16 {
        let lo = self.ctx.borrow_mut().read_cycle(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let hi = self.ctx.borrow_mut().read_cycle(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(1);
        ((hi as u16) << 8) | (lo as u16)
    }

    /// PUSH rr
    ///
    /// Flags: Z N H C
    ///        - - - -
    fn push(&mut self) {
        let value: u16 = self.registers.read16(self.instruction.reg1.unwrap());
        self.push_value(value);
    }

    fn push_value(&mut self, value: u16) {
        let msb = (value >> 8) as u8;
        let lsb = (value & 0xFF) as u8;
        let mut ctx = self.ctx.borrow_mut();
        ctx.tick_cycle();
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        ctx.write_cycle(self.registers.sp, msb);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        ctx.write_cycle(self.registers.sp, lsb);
    }

    /// CCF
    ///
    /// Flags: Z N H C
    ///        - 0 0 *
    fn ccf(&mut self) {
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(!self.registers.cf());
    }

    /// SCF
    ///
    /// Flags: Z N H C
    ///        - 0 0 1
    fn scf(&mut self) {
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(true);
    }

    /// CPL
    ///
    /// Flags: Z N H C
    ///        - 1 1 -
    fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.set_nf(true);
        self.registers.set_hf(true);
    }

    /// DAA
    ///
    /// Flags: Z N H C
    ///        * - 0 *
    ///
    /// The DAA (Decimal Adjust Accumulator) instruction is used to ensure the value in the A register
    /// is a valid BCD (Binary-Coded Decimal) value following a BCD addition or subtraction operation.
    fn daa(&mut self) {
        let mut a = self.fetched_data as u8;
        let mut adjust = 0;
        let mut carry = false;

        if !self.registers.nf() {
            // The previous operation was addition
            if self.registers.hf() || (a & 0x0F) > 9 {
                adjust |= 0x06;
            }
            if self.registers.cf() || a > 0x99 {
                adjust |= 0x60;
                carry = true;
            }
            a = a.wrapping_add(adjust);
        } else {
            // The previous operation was subtraction
            if self.registers.hf() {
                adjust |= 0x06;
            }
            if self.registers.cf() {
                adjust |= 0x60;
            }
            a = a.wrapping_sub(adjust);
        }

        self.registers.a = a;
        self.registers.set_zf(a == 0);
        self.registers.set_hf(false);
        self.registers.set_cf(carry);
    }

    /// ADC sime_scheduled
    ///
    /// Flags: Z N H C
    ///        * 0 * *
    fn adc(&mut self) {
        assert!(self.instruction.reg1.unwrap() == Register::A);

        let value = self.fetched_data as u8;
        let cf = self.registers.cf() as u8;
        let result = self
            .registers
            .read8(Register::A)
            .wrapping_add(value)
            .wrapping_add(cf);
        let half_carry = ((self.registers.read8(Register::A) & 0x0F) + (value & 0x0F) + cf) > 0x0F;
        let carry =
            ((self.registers.read8(Register::A) as u16) + (value as u16) + (cf as u16)) > 0xFF;
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf(half_carry);
        self.registers.set_cf(carry);
        self.registers.write8(Register::A, result);
    }

    /// ADD s
    ///
    /// Flags: Z N H C
    ///        * 0 * *
    fn add(&mut self) {
        let reg1 = self.instruction.reg1.unwrap();

        if reg1.is_16bit() {
            assert!(reg1 == Register::HL);
            let value = self.fetched_data;
            let (result, carry) = self.registers.read16(Register::HL).overflowing_add(value);
            let half_carry =
                ((self.registers.read16(Register::HL) & 0x0FFF) + (value & 0x0FFF)) > 0x0FFF;
            self.registers.set_nf(false);
            self.registers.set_hf(half_carry);
            self.registers.set_cf(carry);
            self.registers.write16(Register::HL, result);
            return;
        }

        assert!(reg1 == Register::A);

        let value = self.fetched_data as u8;
        let (result, carry) = self.registers.read8(Register::A).overflowing_add(value);
        let half_carry = ((self.registers.read8(Register::A) & 0x0F) + (value & 0x0F)) > 0x0F;
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf(half_carry);
        self.registers.set_cf(carry);
        self.registers.write8(Register::A, result);
    }

    /// CP s
    ///
    /// Flags: Z N H C
    ///        * 1 * *
    fn cp(&mut self) {
        let value = self.fetched_data as u8;
        let result = self.registers.read8(Register::A).wrapping_sub(value);
        let carry = self.registers.read8(Register::A) < value;
        let half_carry = (self.registers.read8(Register::A) & 0x0F) < (value & 0x0F);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(true);
        self.registers.set_hf(half_carry);
        self.registers.set_cf(carry);
    }

    /// SBC s
    ///
    /// Flags: Z N H C
    ///        * 1 * *
    fn sbc(&mut self) {
        let value = self.fetched_data as u8;
        let cf = self.registers.cf() as u8;
        let result = self
            .registers
            .read8(Register::A)
            .wrapping_sub(value)
            .wrapping_sub(cf);
        let carry = (self.registers.read8(Register::A) as u16) < (value as u16) + (cf as u16);
        let half_carry = (self.registers.read8(Register::A) & 0x0F) < ((value & 0x0F) + cf);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(true);
        self.registers.set_hf(half_carry);
        self.registers.set_cf(carry);
        self.registers.write8(Register::A, result);
    }

    /// SUB s
    ///
    /// Flags: Z N H C
    ///        * 1 * *
    fn sub(&mut self) {
        let value = self.fetched_data as u8;
        let result = self.registers.read8(Register::A).wrapping_sub(value);
        let carry = self.registers.read8(Register::A) < value;
        let half_carry = (self.registers.read8(Register::A) & 0x0F) < (value & 0x0F);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(true);
        self.registers.set_hf(half_carry);
        self.registers.set_cf(carry);
        self.registers.write8(Register::A, result);
    }

    /// AND s
    ///
    /// Flags: Z N H C
    ///        * 0 1 0
    fn and(&mut self) {
        let result = self.registers.read8(Register::A) & ((self.fetched_data & 0x00FF) as u8);
        self.registers.write8(Register::A, result);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf(true);
        self.registers.set_cf(false);
    }

    /// OR s
    ///
    /// Flags: Z N H C
    ///        * 0 0 0
    fn or(&mut self) {
        let result = self.registers.read8(Register::A) | ((self.fetched_data & 0x00FF) as u8);
        self.registers.write8(Register::A, result);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(false);
    }

    /// XOR s
    ///
    /// Flags: Z N H C
    ///        * 0 0 0
    fn xor(&mut self) {
        let result = self.registers.read8(Register::A) ^ ((self.fetched_data & 0x00FF) as u8);
        self.registers.write8(Register::A, result);
        self.registers.set_zf(result == 0);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(false);
    }

    /// RLA (Rotate Left through Carry A)
    ///
    /// Flags: Z N H C
    ///        0 0 0 *
    fn rla(&mut self) {
        let a_msb = self.registers.a & 0x80;
        self.registers.a = (self.registers.a << 1) | (self.registers.cf() as u8);
        self.registers.set_zf(false);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(a_msb != 0);
    }

    /// RLCA (Rotate Left Circular A)
    ///
    /// Flags: Z N H C
    ///        0 0 0 *
    fn rlca(&mut self) {
        let a_msb = self.registers.a & 0x80;
        self.registers.a = (self.registers.a << 1) | (a_msb >> 7);
        self.registers.set_zf(false);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(a_msb != 0);
    }

    /// RRA (Rotate Right through Carry A)
    ///
    /// Flags: Z N H C
    ///        0 0 0 *
    fn rra(&mut self) {
        let a_lsb = self.registers.a & 1;
        self.registers.a = (self.registers.a >> 1) | ((self.registers.cf() as u8) << 7);
        self.registers.set_zf(false);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(a_lsb != 0);
    }

    /// RRCA (Rotate Right Circular A)
    ///
    /// Flags: Z N H C
    ///        0 0 0 *
    fn rrca(&mut self) {
        let a_lsb = self.registers.a & 1;
        self.registers.a = (self.registers.a >> 1) | (a_lsb << 7);
        self.registers.set_zf(false);
        self.registers.set_nf(false);
        self.registers.set_hf(false);
        self.registers.set_cf(a_lsb != 0);
    }
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CPU register file:\n{}", self.registers)
    }
}
