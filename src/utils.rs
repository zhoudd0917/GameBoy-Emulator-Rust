pub type Byte = u8;
pub type SignedByte = i8;
pub type Address = u16;
pub type Word = u16;

pub fn bytes2word(lsb: Byte, msb: Byte) -> Word {
    (lsb as Word).set_high(msb)
}

pub trait ByteOP {
    fn mask(&self, mask: Byte) -> Byte;
    fn get_low_nibble(&self) -> Byte;
    fn get_high_nibble(&self) -> Byte;
}

impl ByteOP for Byte {
    fn mask(&self, mask: Byte) -> Byte {
        self & mask
    }
    fn get_low_nibble(&self) -> Byte {
        self & 0xF
    }
    fn get_high_nibble(&self) -> Byte {
        (self & 0xF0) >> 4
    }
}

pub trait WordOP {
    fn get_low(&self) -> Byte;
    fn get_high(&self) -> Byte;
    fn set_low(&self, value: Byte) -> Word;
    fn set_high(&self, value: Byte) -> Word;
    fn mask(&self, mask: Word) -> Word;
}

impl WordOP for u16 {
    fn get_low(&self) -> Byte {
        (self & 0xff) as Byte
    }
    fn get_high(&self) -> Byte {
        (self >> 8) as Byte
    }
    fn set_low(&self, value: Byte) -> Word {
        let mut word = self & !0xff;
        word |= value as Word;
        word
    }
    fn set_high(&self, value: Byte) -> Word {
        let mut word = self & 0xff;
        word |= (value as Word) << 8;
        word
    }
    fn mask(&self, mask: Word) -> Word {
        self & mask
    }
}

pub fn byte2stringbit(byte: Byte) -> String {
    format!("{:#010b}", byte)
}

pub fn byte2string(byte: Byte) -> String {
    format!("{:#04X?}", byte)
}

pub fn address2string(address: Address) -> String {
    format!("{:#04X?}", address)
}

pub fn get_flag(flag_byte: Byte, flag: Byte) -> bool {
    assert_eq!(flag.count_ones(), 1);
    (flag_byte & flag) > 0
}

pub fn set_flag(flag_byte: &mut Byte, flag: Byte) {
    assert_eq!(flag.count_ones(), 1);
    *flag_byte |= flag;
}

pub fn set_flag_ref(flag_byte: Byte, flag: Byte) -> Byte {
    assert_eq!(flag.count_ones(), 1);
    flag_byte | flag
}

pub fn reset_flag(flag_byte: &mut Byte, flag: Byte) {
    assert_eq!(flag.count_ones(), 1);
    *flag_byte &= !flag;
}

pub fn reset_all_flags(flag_byte: &mut Byte) {
    *flag_byte = 0;
}
