use std::env;
use std::process;

use dmg01emu::Cartridge;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Provide a ROM file...");
        process::exit(1);
    }

    let rom_file = &args[1];

    println!("Reading {rom_file}");

    if let Err(e) = Cartridge::read(&rom_file) {
        eprintln!("Error reading ROM {e}");
        process::exit(1);
    }
}
