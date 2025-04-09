use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Debug)]
pub struct CartridgeHeader {
    destination: String,
    nintendo_logo: [u8; 48],
    cgb_flag: bool,
    sgb_flag: bool,
    licensee: String,
    title: String,
    rom_size: u32,
    rom_type: u8,
    rom_type_name: String,
    rom_version: u8,
    ram_size: u32,
    header_checksum: u8,
    global_checksum: u16,
}

impl CartridgeHeader {
    pub fn load(rom_contents: &Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let nintendo_logo;

        if let Ok(logo) = rom_contents[0x104..=0x133].try_into() {
            nintendo_logo = logo;
        } else {
            nintendo_logo = [0 as u8; 48];
        }

        Ok(CartridgeHeader {
            destination: String::from(CartridgeHeader::get_destination(rom_contents)),
            nintendo_logo: nintendo_logo,
            cgb_flag: rom_contents[0x0143] == 0x80 || rom_contents[0x0143] == 0xC0,
            sgb_flag: rom_contents[0x146] == 0x03,
            licensee: String::from(CartridgeHeader::get_licensee(rom_contents)),
            title: CartridgeHeader::get_game_title(rom_contents),
            rom_size: CartridgeHeader::get_rom_size(rom_contents),
            rom_type: rom_contents[0x147],
            rom_type_name: String::from(CartridgeHeader::get_rom_type(rom_contents)),
            rom_version: rom_contents[0x14C],
            ram_size: CartridgeHeader::get_ram_size(rom_contents),
            header_checksum: rom_contents[0x14D],
            global_checksum: CartridgeHeader::get_global_checksum(rom_contents),
        })
    }

    pub fn checksum(rom_contents: &Vec<u8>) -> u8 {
        let mut sum: u8 = 0;
        for i in 0x0134..=0x014C {
            // Subtraction with overflow is not associative
            sum = sum.wrapping_sub(rom_contents[i]).wrapping_sub(1);
        }

        return sum;
    }

    fn get_global_checksum(rom_contents: &Vec<u8>) -> u16 {
        (rom_contents[0x14E] as u16) << 8 | (rom_contents[0x14F] as u16)
    }

    fn get_game_title(rom_contents: &Vec<u8>) -> String {
        let bytes = &rom_contents[0x134..=0x143];
        let mut title = String::new();

        for (i, b) in bytes.iter().enumerate() {
            if i > 15 {
                break;
            }

            match char::from_u32(*b as u32) {
                Some(ch) => {
                    if ch != '\0' {
                        title.push(ch);
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        return title;
    }

    fn get_destination(rom_contents: &Vec<u8>) -> &'static str {
        if rom_contents[0x014A] == 0 {
            "Japan"
        } else {
            "Overseas"
        }
    }

    fn get_rom_size(rom_contents: &Vec<u8>) -> u32 {
        let known_sizes: HashMap<u8, u32> = HashMap::from([
            (0x00, 32 * 1024),           // 32 KiB, 2 banks (no banking)
            (0x01, 64 * 1024),           // 64 KiB, 4 banks
            (0x02, 128 * 1024),          // 128 KiB, 8 banks
            (0x03, 256 * 1024),          // 256 KiB, 16 banks
            (0x04, 512 * 1024),          // 512 KiB, 32 banks
            (0x05, 1 * 1024 * 1024),     // 1 MiB, 64 banks
            (0x06, 2 * 1024 * 1024),     // 2 MiB, 128 banks
            (0x07, 4 * 1024 * 1024),     // 4 MiB, 256 banks
            (0x08, 8 * 1024 * 1024),     // 8 MiB, 512 banks
            (0x52, 1_048_576 + 131_072), // 1.1 MiB, 72 banks
            (0x53, 1_048_576 + 262_144), // 1.2 MiB, 80 banks
            (0x54, 1_048_576 + 524_288), // 1.5 MiB, 96 banks
        ]);

        let size_byte = rom_contents[0x148];

        if let Some(size) = known_sizes.get(&size_byte) {
            return *size;
        } else {
            panic!("Unknown cartride size.");
        }
    }

    fn get_ram_size(rom_contents: &Vec<u8>) -> u32 {
        let known_sizes: [u32; 6] = [
            0,
            0,
            8 * 1024,   /*KiB, 1 bank*/
            32 * 1024,  /* 4 banks of 8 KiB each */
            128 * 1024, /* 16 banks of 8 KiB each */
            64 * 1024,  /* 8 banks of 8 KiB each */
        ];

        let size_byte = rom_contents[0x149] as usize;

        if let Some(size) = known_sizes.get(size_byte) {
            return *size;
        } else {
            panic!("Unknown cartridge RAM size.");
        }
    }

    fn get_rom_type(rom_contents: &Vec<u8>) -> &'static str {
        let cartridge_types: HashMap<u8, &'static str> = HashMap::from([
            (0x00, "ROM ONLY"),
            (0x01, "MBC1"),
            (0x02, "MBC1+RAM"),
            (0x03, "MBC1+RAM+BATTERY"),
            (0x05, "MBC2"),
            (0x06, "MBC2+BATTERY"),
            (0x08, "ROM+RAM"),
            (0x09, "ROM+RAM+BATTERY"),
            (0x0B, "MMM01"),
            (0x0C, "MMM01+RAM"),
            (0x0D, "MMM01+RAM+BATTERY"),
            (0x0F, "MBC3+TIMER+BATTERY"),
            (0x10, "MBC3+TIMER+RAM+BATTERY"),
            (0x11, "MBC3"),
            (0x12, "MBC3+RAM"),
            (0x13, "MBC3+RAM+BATTERY"),
            (0x19, "MBC5"),
            (0x1A, "MBC5+RAM"),
            (0x1B, "MBC5+RAM+BATTERY"),
            (0x1C, "MBC5+RUMBLE"),
            (0x1D, "MBC5+RUMBLE+RAM"),
            (0x1E, "MBC5+RUMBLE+RAM+BATTERY"),
            (0x20, "MBC6"),
            (0x22, "MBC7+SENSOR+RUMBLE+RAM+BATTERY"),
            (0xFC, "POCKET CAMERA"),
            (0xFD, "BANDAI TAMA5"),
            (0xFE, "HuC3"),
            (0xFF, "HuC1+RAM+BATTERY"),
        ]);

        let cartridge_type_byte = rom_contents[0x147];
        if let Some(cartridge_type) = cartridge_types.get(&cartridge_type_byte) {
            return cartridge_type;
        } else {
            eprintln!("Unknown cartridge type: 0x{:X}", cartridge_type_byte);
        }

        return "";
    }

    fn get_licensee(rom_contents: &Vec<u8>) -> &'static str {
        let new_licensee_map: HashMap<&'static str, &'static str> = HashMap::from([
            ("00", "None"),
            ("01", "Nintendo Research & Development 1"),
            ("08", "Capcom"),
            ("13", "EA (Electronic Arts)"),
            ("18", "Hudson Soft"),
            ("19", "B-AI"),
            ("20", "KSS"),
            ("22", "Planning Office WADA"),
            ("24", "PCM Complete"),
            ("25", "San-X"),
            ("28", "Kemco"),
            ("29", "SETA Corporation"),
            ("30", "Viacom"),
            ("31", "Nintendo"),
            ("32", "Bandai"),
            ("33", "Ocean Software/Acclaim Entertainment"),
            ("34", "Konami"),
            ("35", "HectorSoft"),
            ("37", "Taito"),
            ("38", "Hudson Soft"),
            ("39", "Banpresto"),
            ("41", "Ubi Soft1"),
            ("42", "Atlus"),
            ("44", "Malibu Interactive"),
            ("46", "Angel"),
            ("47", "Bullet-Proof Software2"),
            ("49", "Irem"),
            ("50", "Absolute"),
            ("51", "Acclaim Entertainment"),
            ("52", "Activision"),
            ("53", "Sammy USA Corporation"),
            ("54", "Konami"),
            ("55", "Hi Tech Expressions"),
            ("56", "LJN"),
            ("57", "Matchbox"),
            ("58", "Mattel"),
            ("59", "Milton Bradley Company"),
            ("60", "Titus Interactive"),
            ("61", "Virgin Games Ltd.3"),
            ("64", "Lucasfilm Games4"),
            ("67", "Ocean Software"),
            ("69", "EA (Electronic Arts)"),
            ("70", "Infogrames5"),
            ("71", "Interplay Entertainment"),
            ("72", "Broderbund"),
            ("73", "Sculptured Software6"),
            ("75", "The Sales Curve Limited7"),
            ("78", "THQ"),
            ("79", "Accolade"),
            ("80", "Misawa Entertainment"),
            ("83", "lozc"),
            ("86", "Tokuma Shoten"),
            ("87", "Tsukuda Original"),
            ("91", "Chunsoft Co.8"),
            ("92", "Video System"),
            ("93", "Ocean Software/Acclaim Entertainment"),
            ("95", "Varie"),
            ("96", "Yonezawa/s’pal"),
            ("97", "Kaneko"),
            ("99", "Pack-In-Video"),
            ("9H", "Bottom Up"),
            ("A4", "Konami (Yu-Gi-Oh!)"),
            ("BL", "MTO"),
            ("DK", "Kodansha"),
        ]);

        let old_licensee_map: HashMap<u8, &'static str> = HashMap::from([
            (0x00, "None"),
            (0x01, "Nintendo"),
            (0x08, "Capcom"),
            (0x09, "HOT-B"),
            (0x0A, "Jaleco"),
            (0x0B, "Coconuts Japan"),
            (0x0C, "Elite Systems"),
            (0x13, "EA (Electronic Arts)"),
            (0x18, "Hudson Soft"),
            (0x19, "ITC Entertainment"),
            (0x1A, "Yanoman"),
            (0x1D, "Japan Clary"),
            (0x1F, "Virgin Games Ltd.3"),
            (0x24, "PCM Complete"),
            (0x25, "San-X"),
            (0x28, "Kemco"),
            (0x29, "SETA Corporation"),
            (0x30, "Infogrames5"),
            (0x31, "Nintendo"),
            (0x32, "Bandai"),
            (
                0x33,
                "Indicates that the New licensee code should be used instead.",
            ),
            (0x34, "Konami"),
            (0x35, "HectorSoft"),
            (0x38, "Capcom"),
            (0x39, "Banpresto"),
            (0x3C, "Entertainment Interactive (stub)"),
            (0x3E, "Gremlin"),
            (0x41, "Ubi Soft1"),
            (0x42, "Atlus"),
            (0x44, "Malibu Interactive"),
            (0x46, "Angel"),
            (0x47, "Spectrum HoloByte"),
            (0x49, "Irem"),
            (0x4A, "Virgin Games Ltd.3"),
            (0x4D, "Malibu Interactive"),
            (0x4F, "U.S. Gold"),
            (0x50, "Absolute"),
            (0x51, "Acclaim Entertainment"),
            (0x52, "Activision"),
            (0x53, "Sammy USA Corporation"),
            (0x54, "GameTek"),
            (0x55, "Park Place13"),
            (0x56, "LJN"),
            (0x57, "Matchbox"),
            (0x59, "Milton Bradley Company"),
            (0x5A, "Mindscape"),
            (0x5B, "Romstar"),
            (0x5C, "Naxat Soft14"),
            (0x5D, "Tradewest"),
            (0x60, "Titus Interactive"),
            (0x61, "Virgin Games Ltd.3"),
            (0x67, "Ocean Software"),
            (0x69, "EA (Electronic Arts)"),
            (0x6E, "Elite Systems"),
            (0x6F, "Electro Brain"),
            (0x70, "Infogrames5"),
            (0x71, "Interplay Entertainment"),
            (0x72, "Broderbund"),
            (0x73, "Sculptured Software6"),
            (0x75, "The Sales Curve Limited7"),
            (0x78, "THQ"),
            (0x79, "Accolade15"),
            (0x7A, "Triffix Entertainment"),
            (0x7C, "MicroProse"),
            (0x7F, "Kemco"),
            (0x80, "Misawa Entertainment"),
            (0x83, "LOZC G."),
            (0x86, "Tokuma Shoten"),
            (0x8B, "Bullet-Proof Software2"),
            (0x8C, "Vic Tokai Corp.16"),
            (0x8E, "Ape Inc.17"),
            (0x8F, "I’Max18"),
            (0x91, "Chunsoft Co.8"),
            (0x92, "Video System"),
            (0x93, "Tsubaraya Productions"),
            (0x95, "Varie"),
            (0x96, "Yonezawa19/S’Pal"),
            (0x97, "Kemco"),
            (0x99, "Arc"),
            (0x9A, "Nihon Bussan"),
            (0x9B, "Tecmo"),
            (0x9C, "Imagineer"),
            (0x9D, "Banpresto"),
            (0x9F, "Nova"),
            (0xA1, "Hori Electric"),
            (0xA2, "Bandai"),
            (0xA4, "Konami"),
            (0xA6, "Kawada"),
            (0xA7, "Takara"),
            (0xA9, "Technos Japan"),
            (0xAA, "Broderbund"),
            (0xAC, "Toei Animation"),
            (0xAD, "Toho"),
            (0xAF, "Namco"),
            (0xB0, "Acclaim Entertainment"),
            (0xB1, "ASCII Corporation or Nexsoft"),
            (0xB2, "Bandai"),
            (0xB4, "Square Enix"),
            (0xB6, "HAL Laboratory"),
            (0xB7, "SNK"),
            (0xB9, "Pony Canyon"),
            (0xBA, "Culture Brain"),
            (0xBB, "Sunsoft"),
            (0xBD, "Sony Imagesoft"),
            (0xBF, "Sammy Corporation"),
            (0xC0, "Taito"),
            (0xC2, "Kemco"),
            (0xC3, "Square"),
            (0xC4, "Tokuma Shoten"),
            (0xC5, "Data East"),
            (0xC6, "Tonkin House"),
            (0xC8, "Koei"),
            (0xC9, "UFL"),
            (0xCA, "Ultra Games"),
            (0xCB, "VAP, Inc."),
            (0xCC, "Use Corporation"),
            (0xCD, "Meldac"),
            (0xCE, "Pony Canyon"),
            (0xCF, "Angel"),
            (0xD0, "Taito"),
            (0xD1, "SOFEL (Software Engineering Lab)"),
            (0xD2, "Quest"),
            (0xD3, "Sigma Enterprises"),
            (0xD4, "ASK Kodansha Co."),
            (0xD6, "Naxat Soft14"),
            (0xD7, "Copya System"),
            (0xD9, "Banpresto"),
            (0xDA, "Tomy"),
            (0xDB, "LJN"),
            (0xDD, "Nippon Computer Systems"),
            (0xDE, "Human Ent."),
            (0xDF, "Altron"),
            (0xE0, "Jaleco"),
            (0xE1, "Towa Chiki"),
            (0xE2, "Yutaka # Needs more info"),
            (0xE3, "Varie"),
            (0xE5, "Epoch"),
            (0xE7, "Athena"),
            (0xE8, "Asmik Ace Entertainment"),
            (0xE9, "Natsume"),
            (0xEA, "King Records"),
            (0xEB, "Atlus"),
            (0xEC, "Epic/Sony Records"),
            (0xEE, "IGS"),
            (0xF0, "A Wave"),
            (0xF3, "Extreme Entertainment"),
            (0xFF, "LJN"),
        ]);

        if rom_contents[0x014B] != 0x33 {
            if let Some(name) = old_licensee_map.get(&rom_contents[0x014B]) {
                return name;
            } else {
                eprintln!(
                    "Invalid old licensee hex code 0x{:X}.",
                    rom_contents[0x014B]
                );
            }
        } else {
            if let Ok(code) = String::from_utf8(rom_contents[0x144..=0x145].to_vec()) {
                if let Some(name) = new_licensee_map.get(code.as_str()) {
                    return name;
                } else {
                    eprintln!(
                        "Invalid new licensee ASCII code [0x{}, 0x{}]",
                        rom_contents[0x144], rom_contents[0x145]
                    );
                }
            } else {
                eprintln!(
                    "Invalid new licensee ASCII code [0x{}, 0x{}]",
                    rom_contents[0x144], rom_contents[0x145]
                );
            }
        }

        return "";
    }
}

pub struct Cartridge {
    pub file: String,
    pub size: u32,
    pub data: Vec<u8>,
    pub header: CartridgeHeader,
}

impl Cartridge {
    pub fn load(file: &str) -> Result<Self, Box<dyn Error>> {
        let rom_contents = fs::read(file)?;

        assert!(rom_contents.len() > 0x14F + 1);

        let rom_header = CartridgeHeader::load(&rom_contents)?;

        assert_eq!(
            CartridgeHeader::checksum(&rom_contents),
            rom_header.header_checksum
        );

        println!("Cartridge Loaded:");
        println!("\t Title    : {}", rom_header.title);
        println!(
            "\t Type     : {} ({})",
            rom_header.rom_type, rom_header.rom_type_name
        );
        println!("\t ROM Size : {} KB", rom_header.rom_size / 1024);
        println!("\t RAM Size : {} KB", rom_header.ram_size / 1024);
        println!(
            "\t LIC Code : {} ({})",
            rom_contents[0x014B], rom_header.licensee
        );
        println!("\t ROM Vers : {}", rom_header.rom_version);

        return Ok(Cartridge {
            file: file.to_string(),
            size: rom_contents.len() as u32,
            data: rom_contents,
            header: rom_header,
        });
    }
}
