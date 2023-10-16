pub mod modrm;

use crate::emulator::modrm::ModRM;
use bit_field::BitField;
use log::info;
use paste::paste;
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

const AL: u8 = EAX;
const AH: u8 = AL + 4;
const CL: u8 = ECX;
const CH: u8 = CL + 4;
const DL: u8 = EDX;
const DH: u8 = DL + 4;
const BL: u8 = EBX;
const BH: u8 = BL + 4;

const CARRY_FLAG: usize = 0;
const ZERO_FLAG: usize = 6;
const SIGN_FLAG: usize = 7;
const OVERFLOW_FLAG: usize = 11;

macro_rules! define_jcc_8 {
    ($cc:stmt, $f:ident) => {
        paste! {
        fn [<j $cc>](&mut self) {
            let diff = self.get_sign_code8(1) as u32;
            if self.$f() {
                self.eip += diff.wrapping_add(2);
            } else {
                self.eip += 2;
            }
        }
        fn [<jn $cc>](&mut self) {
            let diff = self.get_sign_code8(1) as u32;
            if !self.$f() {
                self.eip += diff.wrapping_add(2);
            } else {
                self.eip += 2;
            }
        }
        }
    };
}

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
        info!("EIP = {:08x}, Code = {:02x}", self.eip, code);
        match self.get_code8(0) {
            0x01 => Self::add_rm32_r32,
            0x3b => Self::cmp_r32_rm32,
            0x3c => Self::cmp_al_imm8,
            0x3d => Self::cmp_eax_imm32,
            0x40..=0x47 => Self::inc_r32,
            0x50..=0x57 => Self::push_r32,
            0x58..=0x5f => Self::pop_r32,
            0x68 => Self::push_imm32,
            0x6a => Self::push_imm8,
            0x70 => Self::jo,
            0x71 => Self::jno,
            0x72 => Self::jc,
            0x73 => Self::jnc,
            0x74 => Self::jz,
            0x75 => Self::jnz,
            0x76 => Self::jbe,
            0x77 => Self::jnbe,
            0x78 => Self::js,
            0x79 => Self::jns,
            0x7c => Self::jl,
            0x7d => Self::jnl,
            0x7e => Self::jle,
            0x7f => Self::jnle,
            0x83 => Self::code_83,
            0x88 => Self::mov_rm8_r8,
            0x89 => Self::mov_rm32_r32,
            0x8a => Self::mov_r8_rm8,
            0x8b => Self::mov_r32_rm32,
            0xb0..=0xb7 => Self::mov_r8_imm8,
            0xb8..=0xbf => Self::mov_r32_imm32,
            0xc3 => Self::ret,
            0xc7 => Self::mov_rm32_imm32,
            0xc9 => Self::leave,
            0xe8 => Self::call_rel32,
            0xe9 => Self::near_jump,
            0xeb => Self::short_jump,
            0xec => Self::in_al_dx,
            0xee => Self::out_dx_al,
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

        info!(
            "mod = {}, op = {}, rm = {} ({:02X})",
            modrm.md, modrm.op, modrm.rm, code
        );
        modrm
    }

    fn mov_r32_imm32(&mut self) {
        let reg = self.get_code8(0) - 0xb8;
        let value = self.get_code32(1);
        self.set_register32(reg, value);
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
    fn add_rm32_imm8(&mut self, modrm: &ModRM) {
        let rm32 = self.get_rm32(modrm);
        let imm8 = self.get_sign_code8(0) as u32;
        self.eip += 1;
        self.set_rm32(modrm, rm32.wrapping_add(imm8));
    }
    fn sub_rm32_imm8(&mut self, modrm: &ModRM) {
        let rm32 = self.get_rm32(modrm);
        let imm8 = self.get_sign_code8(0) as u32;
        self.eip += 1;
        let result = (rm32 as u64).wrapping_sub(imm8 as u64);
        self.update_eflags_sub(rm32, imm8, result);
        self.set_rm32(modrm, rm32.wrapping_sub(imm8));
    }
    fn code_83(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        match modrm.op {
            0 => self.add_rm32_imm8(&modrm),
            5 => self.sub_rm32_imm8(&modrm),
            7 => self.cmp_rm32_imm8(&modrm),
            _ => unimplemented!("Not implemented 0x83 /{}", modrm.op),
        }
    }
    fn inc_rm32(&mut self, modrm: &ModRM) {
        let value = self.get_rm32(modrm);
        self.set_rm32(modrm, value.wrapping_add(1));
    }

    fn inc_r32(&mut self) {
        let reg = self.get_code8(0) - 0x40;
        let value = self.get_register32(reg);
        self.set_register32(reg, value.wrapping_add(1));
        self.eip += 1;
    }
    fn dec_rm32(&mut self, modrm: &ModRM) {
        let value = self.get_rm32(modrm);
        self.set_rm32(modrm, value.wrapping_sub(1));
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
    fn cmp_r32_rm32(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let r32 = self.get_r32(&modrm);
        let rm32 = self.get_rm32(&modrm);
        let result = (r32 as u64).wrapping_sub(rm32 as u64);
        self.update_eflags_sub(r32, rm32, result);
    }

    fn cmp_eax_imm32(&mut self) {
        self.eip += 1;
        let value = self.get_code32(1);
        let eax = self.get_register32(EAX);
        let result = (eax as u64).wrapping_sub(value as u64);
        self.update_eflags_sub(value, eax, result);
    }

    fn cmp_rm32_imm8(&mut self, modrm: &ModRM) {
        let rm32 = self.get_rm32(modrm);
        let imm8 = self.get_sign_code8(0) as u32;
        self.eip += 1;
        let result = (rm32 as u64).wrapping_sub(imm8 as u64);
        self.update_eflags_sub(rm32, imm8, result);
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

    define_jcc_8!(c, get_carry);
    define_jcc_8!(z, get_zero);
    define_jcc_8!(s, get_sign);
    define_jcc_8!(o, get_overflow);
    define_jcc_8!(be, get_cond_be);
    define_jcc_8!(l, get_cond_l);
    define_jcc_8!(le, get_cond_le);

    fn get_cond_be(&self) -> bool {
        self.get_carry() || self.get_zero()
    }
    fn get_cond_l(&self) -> bool {
        self.get_sign() != self.get_overflow()
    }
    fn get_cond_le(&self) -> bool {
        self.get_zero() || self.get_sign() != self.get_overflow()
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

    fn get_rm8(&self, modrm: &ModRM) -> u8 {
        if modrm.is_reg() {
            self.get_register8(modrm.rm)
        } else {
            let address = self.calc_memory_address(modrm);
            self.get_memory8(address)
        }
    }

    fn set_rm8(&mut self, modrm: &ModRM, value: u8) {
        if modrm.is_reg() {
            self.set_register8(modrm.rm, value);
        } else {
            let address = self.calc_memory_address(modrm);
            self.set_memory8(address, value);
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
            .for_each(|(i, &b)| self.set_memory8(address + i as u32, b));
    }
    fn set_memory8(&mut self, address: u32, value: u8) {
        self.memory[address as usize] = value;
    }

    fn get_r32(&self, modrm: &ModRM) -> u32 {
        self.get_register32(modrm.op)
    }
    fn set_r32(&mut self, modrm: &ModRM, value: u32) {
        self.set_register32(modrm.op, value);
    }
    fn get_r8(&self, modrm: &ModRM) -> u8 {
        self.get_register8(modrm.op)
    }
    fn set_r8(&mut self, modrm: &ModRM, value: u8) {
        self.set_register8(modrm.op, value);
    }

    fn push_r32(&mut self) {
        let reg = self.get_code8(0) - 0x50;
        self.push32(self.get_register32(reg));
        self.eip += 1;
    }

    fn push_imm32(&mut self) {
        let value = self.get_code32(1);
        self.push32(value);
        self.eip += 5;
    }

    fn push_imm8(&mut self) {
        let value = self.get_code8(1);
        self.push32(value as u32);
        self.eip += 2;
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

    fn in_al_dx(&mut self) {
        let address = (self.get_register32(EDX) & 0xffff) as u16;
        let value = self.io_in8(address);
        self.set_register8(AL, value);
        self.eip += 1;
    }

    fn out_dx_al(&mut self) {
        let address = (self.get_register32(EDX) & 0xffff) as u16;
        let value = self.get_register8(AL);
        self.io_out8(address, value);
        self.eip += 1;
    }

    fn update_eflags_sub(&mut self, v1: u32, v2: u32, result: u64) {
        let sign1 = v1.get_bit(31);
        let sign2 = v2.get_bit(31);
        let signr = result.get_bit(31);

        self.set_carry(result >> 32 > 0);
        self.set_zero(result == 0);
        self.set_sign(signr);
        self.set_overflow(sign1 != sign2 && sign1 != signr);
    }

    fn get_flag(&self, flag: usize) -> bool {
        self.eflags.get_bit(flag)
    }
    fn get_carry(&self) -> bool {
        self.eflags.get_bit(CARRY_FLAG)
    }
    fn get_zero(&self) -> bool {
        self.eflags.get_bit(ZERO_FLAG)
    }
    fn get_sign(&self) -> bool {
        self.eflags.get_bit(SIGN_FLAG)
    }
    fn get_overflow(&self) -> bool {
        self.eflags.get_bit(OVERFLOW_FLAG)
    }
    fn set_carry(&mut self, is_carry: bool) {
        self.eflags.set_bit(CARRY_FLAG, is_carry);
    }
    fn set_zero(&mut self, is_zero: bool) {
        self.eflags.set_bit(ZERO_FLAG, is_zero);
    }
    fn set_sign(&mut self, is_sign: bool) {
        self.eflags.set_bit(SIGN_FLAG, is_sign);
    }
    fn set_overflow(&mut self, is_overflow: bool) {
        self.eflags.set_bit(OVERFLOW_FLAG, is_overflow);
    }
    fn io_in8(&self, address: u16) -> u8 {
        match address {
            0x03f8 => {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).expect("stdio is dead");
                buf.as_bytes()[0]
            }
            _ => 0,
        }
    }
    fn io_out8(&self, address: u16, value: u8) {
        match address {
            0x03f8 => {
                if value.is_ascii() {
                    print!("{}", value as char);
                } else {
                    print!("{:02x}", value);
                }
            }
            _ => {}
        }
    }

    fn mov_r8_imm8(&mut self) {
        let reg = self.get_code8(0) - 0xb0;
        let value = self.get_code8(1);
        self.set_register8(reg, value);
        self.eip += 2;
    }
    fn mov_rm8_r8(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let r8 = self.get_r8(&modrm);
        self.set_rm8(&modrm, r8);
    }

    fn mov_r8_rm8(&mut self) {
        self.eip += 1;
        let modrm = self.parse_modrm();
        let rm8 = self.get_rm8(&modrm);
        self.set_r8(&modrm, rm8);
    }

    fn cmp_al_imm8(&mut self) {
        let value = self.get_code8(1);
        let al = self.get_register8(AL);
        let result = (al as u64).wrapping_sub(value as u64);
        self.update_eflags_sub(al as u32, value as u32, result);
        self.eip += 2;
    }

    fn get_register8(&self, index: u8) -> u8 {
        if index < 4 {
            (self.get_register32(index) & 0xff) as u8
        } else {
            (self.get_register32(index - 4) & 0xff00) as u8
        }
    }
    fn set_register8(&mut self, index: u8, value: u8) {
        if index < 4 {
            let r = self.get_register32(index) & 0xffffff00;
            self.set_register32(index, r | value as u32);
        } else {
            let r = self.get_register32(index - 4) & 0xffff00ff;
            self.set_register32(index, r | ((value as u32) << 8));
        }
    }
}
