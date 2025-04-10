#[derive(Debug)]
pub struct CPU {}

impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {}
    }

    pub fn step(&self) -> bool {
        false
    }
}
