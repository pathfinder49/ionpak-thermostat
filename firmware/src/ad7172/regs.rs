use byteorder::{BigEndian, ByteOrder};
use bit_field::BitField;

use super::*;

pub trait Register {
    type Data: RegisterData;
    fn address(&self) -> u8;
}
pub trait RegisterData {
    fn empty() -> Self;
    fn as_mut(&mut self) -> &mut [u8];
}

macro_rules! def_reg {
    ($Reg: ident, $reg: ident, $addr: expr, $size: expr) => {
        pub struct $Reg;
        impl Register for $Reg {
            type Data = $reg::Data;
            fn address(&self) -> u8 {
                $addr
            }
        }
        mod $reg {
            pub struct Data(pub [u8; $size]);
            impl super::RegisterData for Data {
                fn empty() -> Self {
                    Data([0; $size])
                }
                fn as_mut(&mut self) -> &mut [u8] {
                    &mut self.0
                }
            }
        }
    };
    ($Reg: ident, $index: ty, $reg: ident, $addr: expr, $size: expr) => {
        struct $Reg { pub index: $index, }
        impl Register for $Reg {
            type Data = $reg::Data;
            fn address(&self) -> u8 {
                $addr + (self.index as u8)
            }
        }
        mod $reg {
            pub struct Data(pub [u8; $size]);
            impl super::RegisterData for Data {
                fn empty() -> Self {
                    Data([0; $size])
                }
                fn as_mut(&mut self) -> &mut [u8] {
                    &mut self.0
                }
            }
        }
    }
}

def_reg!(Status, status, 0x00, 1);
impl status::Data {
    /// Is there new data to read?
    pub fn ready(&self) -> bool {
        ! self.0[0].get_bit(7)
    }

    /// Channel for which data is ready
    pub fn channel(&self) -> u8 {
        self.0[0].get_bits(0..=1)
    }

    pub fn adc_error(&self) -> bool {
        self.0[0].get_bit(6)
    }

    pub fn crc_error(&self) -> bool {
        self.0[0].get_bit(5)
    }

    pub fn reg_error(&self) -> bool {
        self.0[0].get_bit(4)
    }
}

def_reg!(IfMode, if_mode, 0x02, 2);
impl if_mode::Data {
    pub fn set_crc(&mut self, mode: ChecksumMode) {
        self.0[1].set_bits(2..=3, mode as u8);
    }
}

def_reg!(Data, data, 0x04, 3);
impl data::Data {
    pub fn data(&self) -> u32 {
        (u32::from(self.0[0]) << 16) |
        (u32::from(self.0[1]) << 8) |
        u32::from(self.0[2])
    }
}

def_reg!(Id, id, 0x07, 2);
impl id::Data {
    pub fn id(&self) -> u16 {
        BigEndian::read_u16(&self.0)
    }
}

def_reg!(Channel, u8, channel, 0x10, 2);
impl channel::Data {
    pub fn enabled(&self) -> bool {
        self.0[0].get_bit(7)
    }

    pub fn set_enabled(&mut self, value: bool) {
        self.0[0].set_bit(7, value);
    }
}

def_reg!(SetupCon, u8, setup_con, 0x20, 2);

def_reg!(FiltCon, u8, filt_con, 0x80, 2);

// #[allow(unused)]
// #[derive(Clone, Copy)]
// #[repr(u8)]
// pub enum Register {
//     Status = 0x00,
//     AdcMode = 0x01,
//     IfMode = 0x02,
//     RegCheck = 0x03,
//     Data = 0x04,
//     GpioCon = 0x06,
//     Id = 0x07,
//     Ch0 = 0x10,
//     Ch1 = 0x11,
//     Ch2 = 0x12,
//     Ch3 = 0x13,
//     SetupCon0 = 0x20,
//     SetupCon1 = 0x21,
//     SetupCon2 = 0x22,
//     SetupCon3 = 0x23,
//     FiltCon0 = 0x28,
//     FiltCon1 = 0x29,
//     FiltCon2 = 0x2A,
//     FiltCon3 = 0x2B,
//     Offset0 = 0x30,
//     Offset1 = 0x31,
//     Offset2 = 0x32,
//     Offset3 = 0x33,
//     Gain0 = 0x38,
//     Gain1 = 0x39,
//     Gain2 = 0x3A,
//     Gain3 = 0x3B,
// }
