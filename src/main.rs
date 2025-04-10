use std::env;
use std::process;

use dmg01emu::emu::Emulator;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Provide a ROM file...");
        process::exit(1);
    }

    let rom_file = &args[1];

    println!("Reading {rom_file}");

    let mut dmg_emu = Emulator::new();

    if let Err(e) = dmg_emu.run(rom_file) {
        eprintln!("Error running emulator {e}");
        process::exit(1);
    }
}
