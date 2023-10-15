use bit_field::BitField;

#[derive(Debug)]
pub struct ModRM {
    /// mod
    pub md: u8,
    /// oprand
    pub op: u8,
    /// register index
    pub rm: u8,
    /// scale index base
    pub sib: u8,
    /// displacement
    pub disp: i32,
}

impl ModRM {
    pub fn from_code(code: u8) -> ModRM {
        ModRM {
            md: code.get_bits(6..8),
            op: code.get_bits(3..6),
            rm: code.get_bits(0..3),
            sib: 0,
            disp: 0,
        }
    }

    pub fn is_reg(&self) -> bool {
        self.md == 0b11
    }

    pub fn has_sib(&self) -> bool {
        self.md != 3 && self.rm == 0b100
    }

    pub fn has_disp8(&self) -> bool {
        self.md == 0b01
    }

    pub fn has_disp32(&self) -> bool {
        self.md == 0b10 || (self.md == 0b00 && self.rm == 0b101)
    }

    pub fn set_sib(&mut self, sib: u8) {
        self.sib = sib;
    }

    pub fn set_disp8(&mut self, disp: i8) {
        self.disp = disp as i32;
    }

    pub fn set_disp32(&mut self, disp: i32) {
        self.disp = disp;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn parse_modrm() {
        let esp_ebp = 0xec;
        let modrm = ModRM::from_code(esp_ebp);
        assert_eq!(modrm.md, 0b11);
        assert_eq!(modrm.op, 0b101);
        assert_eq!(modrm.rm, 0b100);
        assert!(!modrm.has_sib());
        assert!(!modrm.has_disp32());
        assert!(!modrm.has_disp8());
    }
}
