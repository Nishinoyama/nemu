pub mod modrm;

use crate::emulator::modrm::ModRM;
use std::num::Wrapping;

const REGISTER_COUNT: usize = 8;

const EAX: u8 = 0;
const ECX: u8 = 1;
const EDX: u8 = 2;
const EBX: u8 = 3;
const ESP: u8 = 4;
const EBP: u8 = 5;
const ESI: u8 = 6;
const EDI: u8 = 7;

pub struct Emulator {
    /// general purpose registers
    pub registers: [u32; REGISTER_COUNT],
    /// eflags register
    pub eflags: u32,
    /// program counter
    pub eip: Wrapping<u32>,
    /// memory
    pub memory: Vec<u8>,
}

impl Emulator {
    pub fn new(size: usize, eip: u32, esp: u32) -> Emulator {
        let mut emulator = Emulator {
            registers: [0; REGISTER_COUNT],
            eflags: 0,
            eip: Wrapping(eip),
            memory: vec![0; size],
        };
        emulator.registers[ESP as usize] = esp;
        emulator
    }

    pub fn instruction(&mut self) -> fn(&mut Emulator) {
        let code = self.get_code8(0);
        eprintln!("EIP = {:08x}, Code = {:02x}", self.eip, code);
        match self.get_code8(0) {
            0x01 => Self::add_rm32_r32,
            0x55..=0x58 => Self::push_r32,
            0x5d..=0x5f => Self::pop_r32,
            0x83 => Self::code_83,
            0x89 => Self::mov_rm32_r32,
            0x8b => Self::mov_r32_rm32,
            0xb8..=0xbf => Self::mov_r32_imm32,
            0xc3 => Self::ret,
            0xc7 => Self::mov_rm32_imm32,
            0xc9 => Self::leave,
            0xeb => Self::short_jump,
            0xe8 => Self::call_rel32,
            0xe9 => Self::near_jump,
            0xff => Self::code_ff,
            _ => unimplemented!("Not implemented code: {:02x}", code),
        }
    }

    pub fn parse_modrm(&mut self) -> ModRM {
        let code = self.get_code8(0);
        let mut modrm = ModRM::from_code(code);
        self.eip += 1;

        if modrm.has_sib() {
            modrm.set_sib(self.get_code8(0));
            self.eip += 1;
        }

        if modrm.has_disp32() {
            modrm.set_disp32(self.get_sign_code32(0));
            self.eip += 4;
        } else if modrm.has_disp8() {
            modrm.set_disp8(self.get_sign_code8(0));
            self.eip += 1;
        }

        eprintln!(
            "mod = {}, op = {}, rm = {} ({:02X})",
            modrm.md, modrm.op, modrm.rm, code
        );
        modrm
    }

    fn mov_r32_imm32(&mut self) {
        let reg = self.get_code8(0) - 0xb8;
        let value = self.get_code32(1);
        self.registers[reg as usize] = value;
        self.eip += 5;
    }

    fn mov_rm32_imm32(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let value = self.get_code32(0);
        self.eip += 4;
        self.set_rm32(&modrm, value);
    }
    fn mov_rm32_r32(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let r32 = self.get_r32(&modrm);
        self.set_rm32(&modrm, r32);
    }
    fn mov_r32_rm32(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let rm32 = self.get_rm32(&modrm);
        self.set_r32(&modrm, rm32);
    }
    fn add_rm32_r32(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let r32 = self.get_r32(&modrm);
        let rm32 = self.get_rm32(&modrm);
        self.set_rm32(&modrm, rm32.wrapping_add(r32));
    }
    fn sub_rm32_imm8(&mut self, modrm: &ModRM) {
        let rm32 = self.get_rm32(modrm);
        let imm8 = self.get_sign_code8(0) as u32;
        self.eip += 1;
        self.set_rm32(modrm, rm32.wrapping_sub(imm8));
    }
    fn code_83(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        match modrm.op {
            5 => self.sub_rm32_imm8(&modrm),
            _ => unimplemented!("Not implemented 0x83 /{}", modrm.op),
        }
    }
    fn inc_rm32(&mut self, modrm: &ModRM) {
        let value = self.get_rm32(&modrm);
        self.set_rm32(&modrm, value.wrapping_add(1));
    }
    fn dec_rm32(&mut self, modrm: &ModRM) {
        let value = self.get_rm32(&modrm);
        self.set_rm32(&modrm, value.wrapping_sub(1));
    }
    fn code_ff(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        match modrm.op {
            0 => self.inc_rm32(&modrm),
            1 => self.dec_rm32(&modrm),
            _ => unimplemented!("Not implemented 0xff /{}", modrm.op),
        }
    }

    fn short_jump(&mut self) {
        let diff = self.get_sign_code8(1);
        self.eip = Wrapping(
            ((self.eip.0 as i32)
                .wrapping_add(diff as i32)
                .wrapping_add(2)) as u32,
        );
    }

    fn near_jump(&mut self) {
        let diff = self.get_code32(1);
        self.eip += diff.wrapping_add(5);
    }

    fn get_code8(&self, index: usize) -> u8 {
        self.memory[(self.eip + Wrapping(index as u32)).0 as usize]
    }

    fn get_sign_code8(&self, index: usize) -> i8 {
        self.get_code8(index) as i8
    }

    fn get_code32(&self, index: usize) -> u32 {
        u32::from_le_bytes([
            self.get_code8(index),
            self.get_code8(index + 1),
            self.get_code8(index + 2),
            self.get_code8(index + 3),
        ])
    }

    fn get_sign_code32(&self, index: usize) -> i32 {
        self.get_code32(index) as i32
    }

    pub fn dump(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("EIP = {:08x}\n", self.eip));
        s.push_str(&self.dump_registers());
        s.push_str(&self.dump_eflags());
        s
    }
    fn dump_registers(&self) -> String {
        let mut s = String::new();
        for i in 0..REGISTER_COUNT {
            s.push_str(&format!(
                "{} = {:08x}\n",
                Self::register_name(i),
                self.registers[i]
            ));
        }
        s
    }
    fn dump_eflags(&self) -> String {
        format!("EFLAGS = {:08x}\n", self.eflags)
    }
    fn register_name(index: usize) -> &'static str {
        match index {
            0 => "EAX",
            1 => "ECX",
            2 => "EDX",
            3 => "EBX",
            4 => "ESP",
            5 => "EBP",
            6 => "ESI",
            7 => "EDI",
            _ => unreachable!(),
        }
    }

    fn get_rm32(&self, modrm: &ModRM) -> u32 {
        if modrm.is_reg() {
            self.get_register32(modrm.rm)
        } else {
            let address = self.calc_memory_address(modrm);
            self.get_memory32(address)
        }
    }

    fn set_rm32(&mut self, modrm: &ModRM, value: u32) {
        if modrm.is_reg() {
            self.set_register32(modrm.rm, value);
        } else {
            let address = self.calc_memory_address(modrm);
            self.set_memory32(address, value);
        }
    }
    fn set_register32(&mut self, reg: u8, value: u32) {
        self.registers[reg as usize] = value;
    }
    fn get_register32(&self, reg: u8) -> u32 {
        self.registers[reg as usize]
    }
    fn calc_memory_address(&self, modrm: &ModRM) -> u32 {
        match modrm.md {
            0 => {
                if modrm.rm == 4 {
                    unimplemented!("Not implemented ModRM mod = 0, rm = 4");
                } else if modrm.rm == 5 {
                    modrm.disp as u32
                } else {
                    self.get_register32(modrm.rm)
                }
            }
            1 => {
                if modrm.rm == 4 {
                    unimplemented!("Not implemented ModRM mod = 1, rm = 4");
                } else {
                    self.get_register32(modrm.rm)
                        .wrapping_add(modrm.disp as u32)
                }
            }
            2 => {
                if modrm.rm == 4 {
                    unimplemented!("Not implemented ModRM mod = 2, rm = 4");
                } else {
                    self.get_register32(modrm.rm)
                        .wrapping_add(modrm.disp as u32)
                }
            }
            3 => {
                unimplemented!("Not implemented ModRM mod = 3");
            }
            _ => unreachable!(),
        }
    }
    fn get_memory32(&self, address: u32) -> u32 {
        u32::from_le_bytes([
            self.get_memory8(address),
            self.get_memory8(address + 1),
            self.get_memory8(address + 2),
            self.get_memory8(address + 3),
        ])
    }
    fn get_memory8(&self, address: u32) -> u8 {
        self.memory[address as usize]
    }
    fn set_memory32(&mut self, address: u32, value: u32) {
        value
            .to_le_bytes()
            .iter()
            .enumerate()
            .for_each(|(i, &b)| self.set_memory8(address + i as u32, b as u32));
    }
    fn set_memory8(&mut self, address: u32, value: u32) {
        self.memory[address as usize] = value as u8;
    }

    fn get_r32(&self, modrm: &ModRM) -> u32 {
        self.get_register32(modrm.op)
    }
    fn set_r32(&mut self, modrm: &ModRM, value: u32) {
        self.set_register32(modrm.op, value);
    }

    fn push_r32(&mut self) {
        let reg = self.get_code8(0) - 0x50;
        self.push32(self.get_register32(reg));
        self.eip += 1;
    }

    fn push32(&mut self, value: u32) {
        let address = self.get_register32(ESP) - 4;
        self.set_register32(ESP, address);
        self.set_memory32(address, value);
    }

    fn pop_r32(&mut self) {
        let reg = self.get_code8(0) - 0x58;
        let value = self.pop32();
        self.set_register32(reg, value);
        self.eip += 1;
    }

    fn pop32(&mut self) -> u32 {
        let address = self.get_register32(ESP);
        let value = self.get_memory32(address);
        self.set_register32(ESP, address + 4);
        value
    }

    fn call_rel32(&mut self) {
        let diff = self.get_sign_code32(1);
        self.push32(self.eip.0 + 5);
        self.eip += (diff + 5) as u32;
    }

    fn ret(&mut self) {
        let address = self.pop32();
        self.eip = Wrapping(address);
    }

    fn leave(&mut self) {
        let ebp = self.get_register32(EBP);
        self.set_register32(ESP, ebp);
        let value = self.pop32();
        self.set_register32(EBP, value);
        self.eip += 1;
    }
}
