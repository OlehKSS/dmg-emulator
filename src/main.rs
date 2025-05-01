use std::env;
use std::process;

use dmgemu::emu::Emulator;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Provide a ROM file...");
        process::exit(1);
    }

    let rom_file = &args[1];

    println!("Reading {rom_file}");

    if let Err(e) = Emulator::run(rom_file) {
        eprintln!("Error running emulator {e}");
        process::exit(1);
    }
}
