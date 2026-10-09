#[derive(Debug)]
pub struct Gamepad {
    pub start: bool,
    pub select: bool,
    pub a: bool,
    pub b: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    button_sel: bool,
    dir_sel: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Key {
    A,
    B,
    Down,
    Left,
    Right,
    Select,
    Start,
    Up,
}

impl Gamepad {
    pub fn new() -> Self {
        Gamepad {
            start: false,
            select: false,
            a: false,
            b: false,
            up: false,
            down: false,
            left: false,
            right: false,
            button_sel: false,
            dir_sel: false,
        }
    }

    pub fn button_selected(&self) -> bool {
        self.button_sel
    }

    pub fn direction_selected(&self) -> bool {
        self.dir_sel
    }

    pub fn key_down(&mut self, key: Key) {
        match key {
            Key::A => self.a = true,
            Key::B => self.b = true,
            Key::Down => self.down = true,
            Key::Left => self.left = true,
            Key::Right => self.right = true,
            Key::Select => self.select = true,
            Key::Start => self.start = true,
            Key::Up => self.up = true,
        }
    }

    pub fn key_up(&mut self, key: Key) {
        match key {
            Key::A => self.a = false,
            Key::B => self.b = false,
            Key::Down => self.down = false,
            Key::Left => self.left = false,
            Key::Right => self.right = false,
            Key::Select => self.select = false,
            Key::Start => self.start = false,
            Key::Up => self.up = false,
        }
    }

    pub fn set_selection(&mut self, value: u8) {
        self.button_sel = (value & (1 << 5)) != 0;
        self.dir_sel = (value & (1 << 4)) != 0;
    }

    pub fn get_output(&self) -> u8 {
        let mut output = 0xCF;

        if !self.button_selected() {
            if self.start {
                output &= !(1 << 3);
            }

            if self.select {
                output &= !(1 << 2);
            }

            if self.b {
                output &= !(1 << 1);
            }

            if self.a {
                output &= !1;
            }
        }

        if !self.direction_selected() {
            if self.down {
                output &= !(1 << 3);
            }

            if self.up {
                output &= !(1 << 2);
            }

            if self.left {
                output &= !(1 << 1);
            }

            if self.right {
                output &= !1;
            }
        }

        output
    }
}

impl Default for Gamepad {
    fn default() -> Self {
        Self::new()
    }
}
