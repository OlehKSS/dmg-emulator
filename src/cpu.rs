#[derive(Debug)]
pub struct CPU {}

impl CPU {
    pub fn new() -> Self {
        CPU {}
    }

    pub fn step(&self) -> bool {
        false
    }
}
