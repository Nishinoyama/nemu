use log::info;
use std::fs::File;
use std::io::Read;

use crate::emulator::Emulator;

pub mod emulator;

fn main() -> std::io::Result<()> {
    env_logger::init();
    let mut emu = Emulator::new(0x4_000_000, 0x7c00, 0x7c00);
    let mut file = File::open("./tolset_p86/exec-io-test/select.bin")?;
    let mut binary = Vec::new();
    file.read_to_end(&mut binary).unwrap();
    for (i, &code) in binary.iter().enumerate() {
        emu.memory[i + 0x7c00] = code;
    }

    loop {
        let instruction = emu.instruction();
        instruction(&mut emu);
        if emu.eip.0 == 0 {
            break;
        }
    }

    info!("Program terminated successfully.");
    info!("{}", emu.dump());
    Ok(())
}
