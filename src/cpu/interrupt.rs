use bitflags::bitflags;

bitflags!(
    pub struct InterruptFlag: u8 {
        const VBLANK = 0b0;
        const LCD = 0b10;
        const TIMER = 0b100;
        const SERIAL = 0b1000;
        const JOYPAD = 0b1_0000;
    }

);

impl InterruptFlag {
    pub fn highest_priority(&self) -> InterruptFlag {
        InterruptFlag::from_bits_truncate(isolate_rightmost_one(self.bits()))
    }
}

pub fn get_hadler_address(f: InterruptFlag) -> u16 {
    let high_f = f.highest_priority();

    if high_f.contains(InterruptFlag::VBLANK) {
        return 0x40;
    } else if high_f.contains(InterruptFlag::LCD) {
        return 0x48;
    } else if high_f.contains(InterruptFlag::TIMER) {
        return 0x50;
    } else if high_f.contains(InterruptFlag::SERIAL) {
        return 0x58;
    } else if high_f.contains(InterruptFlag::JOYPAD) {
        return 0x60;
    }

    panic!("Invalid interrup flag.")
}

fn isolate_rightmost_one(f: u8) -> u8 {
    // Unsigned negation, -f
    let neg_f = (!f).wrapping_add(1);
    // The bitwise AND operation isolates the rightmost 1 bit in x.
    // The two's complement negation (-x) flips all bits after the rightmost 1 bit in x and leaves the rest unchanged.
    f & neg_f
}
