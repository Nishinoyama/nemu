pub mod emulator;

use crate::emulator::Emulator;
use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    let mut emu = Emulator::new(0x4_000_000, 0x7c00, 0x7c00);
    let mut file = File::open("./tolset_p86/exec-c-test/test.bin")?;
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

    println!("Program terminated successfully.");
    println!("{}", emu.dump());
    Ok(())
}
