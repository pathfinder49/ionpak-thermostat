use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::spi::Transfer;
use byteorder::{BigEndian, ByteOrder};
use bit_field::BitField;

trait Register {
    type Data: RegisterData;
    fn address(&self) -> u8;
}
trait RegisterData {
    fn empty() -> Self;
    fn as_mut(&mut self) -> &mut [u8];
}

macro_rules! def_reg {
    ($Reg: ident, $reg: ident, $addr: expr, $size: expr) => {
        struct $Reg;
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
    fn ready(&self) -> bool {
        ! self.0[0].get_bit(7)
    }

    /// Channel for which data is ready
    fn channel(&self) -> u8 {
        self.0[0].get_bits(0..=1)
    }

    fn adc_error(&self) -> bool {
        self.0[0].get_bit(6)
    }

    fn crc_error(&self) -> bool {
        self.0[0].get_bit(5)
    }

    fn reg_error(&self) -> bool {
        self.0[0].get_bit(4)
    }
}

def_reg!(IfMode, if_mode, 0x02, 2);
impl if_mode::Data {
    fn set_crc(&mut self, mode: ChecksumMode) {
        self.0[1].set_bits(2..=3, mode as u8);
    }
}

def_reg!(Data, data, 0x04, 3);
impl data::Data {
    fn data(&self) -> u32 {
        (u32::from(self.0[0]) << 16) |
        (u32::from(self.0[1]) << 8) |
        u32::from(self.0[2])
    }
}

def_reg!(Id, id, 0x07, 2);
impl id::Data {
    fn id(&self) -> u16 {
        BigEndian::read_u16(&self.0)
    }
}

def_reg!(Channel, u8, channel, 0x10, 2);
impl channel::Data {
    fn enabled(&self) -> bool {
        self.0[0].get_bit(7)
    }

    fn set_enabled(&mut self, value: bool) {
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

#[repr(u8)]
pub enum Input {
    Ain0 = 0,
    Ain1 = 1,
    Ain2 = 2,
    Ain3 = 3,
    Ain4 = 4,
    TemperaturePos = 17,
    TemperatureNeg = 18,
    AnalogSupplyPos = 19,
    AnalogSupplyNeg = 20,
    RefPos = 21,
    RefNeg = 22,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AdcError<SPI> {
    SPI(SPI),
    ChecksumMismatch(Option<u8>, Option<u8>),
}

impl<SPI> From<SPI> for AdcError<SPI> {
    fn from(e: SPI) -> Self {
        AdcError::SPI(e)
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ChecksumMode {
    Off = 0b00,
    /// Seems much less reliable than `Crc`
    Xor = 0b01,
    Crc = 0b10,
}

struct Checksum {
    mode: ChecksumMode,
    state: u8,
}

impl Checksum {
    pub fn new(mode: ChecksumMode) -> Self {
        Checksum { mode, state: 0 }
    }
    pub fn feed(&mut self, input: u8) {
        match self.mode {
            ChecksumMode::Off => {},
            ChecksumMode::Xor => self.state ^= input,
            ChecksumMode::Crc => {
                for i in 0..8 {
                    let input_mask = 0x80 >> i;
                    self.state = (self.state << 1) ^
                        if ((self.state & 0x80) != 0) != ((input & input_mask) != 0) {
                            0x07 /* x8 + x2 + x + 1 */
                        } else {
                            0
                        };
                }
            }
        }
    }
    pub fn result(&self) -> Option<u8> {
        match self.mode {
            ChecksumMode::Off => None,
            _ => Some(self.state)
        }
    }
}

/// AD7172-2 implementation
///
/// [Manual](https://www.analog.com/media/en/technical-documentation/data-sheets/AD7172-2.pdf)
pub struct Adc<SPI: Transfer<u8>, NSS: OutputPin> {
    spi: SPI,
    nss: NSS,
    checksum_mode: ChecksumMode,
}

impl<SPI: Transfer<u8>, NSS: OutputPin> Adc<SPI, NSS> {
    pub fn new(spi: SPI, mut nss: NSS) -> Result<Self, SPI::Error> {
        let _ = nss.set_high();
        let mut adc = Adc {
            spi, nss,
            checksum_mode: ChecksumMode::Off,
        };
        adc.reset()?;

        Ok(adc)
    }

    /// `0x00DX` for AD7271-2
    pub fn identify(&mut self) -> Result<u16, AdcError<SPI::Error>> {
        self.read_reg(&Id)
            .map(|id| id.id())
    }

    pub fn set_checksum_mode(&mut self, mode: ChecksumMode) -> Result<(), AdcError<SPI::Error>> {
        let mut ifmode = self.read_reg(&IfMode)?;
        ifmode.set_crc(mode);
        self.checksum_mode = mode;
        self.write_reg(&IfMode, &mut ifmode)?;
        Ok(())
    }

    // pub fn setup(&mut self) -> Result<(), SPI::Error> {
    //     let mut buf = [0, 0, 0];
    //     adc.write_reg(Register::AdcMode, &mut buf)?;
    //     let mut buf = [0, 1, 0];
    //     adc.write_reg(Register::IfMode, &mut buf)?;
    //     let mut buf = [0, 0, 0];
    //     adc.write_reg(Register::GpioCon, &mut buf)?;

    //     Ok(())
    // }

    /// Returns the channel the data is from
    pub fn data_ready(&mut self) -> Result<Option<u8>, AdcError<SPI::Error>> {
        self.read_reg(&Status)
            .map(|status| {
                if status.ready() {
                    Some(status.channel())
                } else {
                    None
                }
            })
    }

    /// Get data
    pub fn read_data(&mut self) -> Result<u32, AdcError<SPI::Error>> {
        self.read_reg(&Data)
            .map(|data| data.data())
    }

    fn read_reg<R: Register>(&mut self, reg: &R) -> Result<R::Data, AdcError<SPI::Error>> {
        let mut reg_data = R::Data::empty();
        let address = 0x40 | reg.address();
        let mut checksum = Checksum::new(self.checksum_mode);
        checksum.feed(address);
        let checksum_out = checksum.result();
        let checksum_in = self.transfer(address, reg_data.as_mut(), checksum_out)?;
        for &mut b in reg_data.as_mut() {
            checksum.feed(b);
        }
        let checksum_expected = checksum.result();
        if checksum_expected != checksum_in {
            return Err(AdcError::ChecksumMismatch(checksum_expected, checksum_in));
        }
        Ok(reg_data)
    }

    fn write_reg<R: Register>(&mut self, reg: &R, reg_data: &mut R::Data) -> Result<(), AdcError<SPI::Error>> {
        let address = reg.address();
        let mut checksum = Checksum::new(match self.checksum_mode {
            ChecksumMode::Off => ChecksumMode::Off,
            // write checksums are always crc
            ChecksumMode::Xor => ChecksumMode::Crc,
            ChecksumMode::Crc => ChecksumMode::Crc,
        });
        checksum.feed(address);
        for &mut b in reg_data.as_mut() {
            checksum.feed(b);
        }
        let checksum_out = checksum.result();
        self.transfer(address, reg_data.as_mut(), checksum_out)?;
        Ok(())
    }

    fn update_reg<R, F, A>(&mut self, reg: &R, f: F) -> Result<A, AdcError<SPI::Error>>
    where
        R: Register,
        F: FnOnce(&mut R::Data) -> A,
    {
        let mut reg_data = self.read_reg(reg)?;
        let result = f(&mut reg_data);
        self.write_reg(reg, &mut reg_data)?;
        Ok(result)
    }

    pub fn reset(&mut self) -> Result<(), SPI::Error> {
        let mut buf = [0xFFu8; 8];
        let _ = self.nss.set_low();
        let result = self.spi.transfer(&mut buf);
        let _ = self.nss.set_high();
        result?;
        Ok(())
    }

    fn transfer<'w>(&mut self, addr: u8, reg_data: &'w mut [u8], checksum: Option<u8>) -> Result<Option<u8>, SPI::Error> {
        let mut addr_buf = [addr];

        let _ = self.nss.set_low();
        let result = match self.spi.transfer(&mut addr_buf) {
            Ok(_) => self.spi.transfer(reg_data),
            Err(e) => Err(e),
        };
        let result = match (result, checksum) {
            (Ok(_),None) =>
                Ok(None),
            (Ok(_), Some(checksum_out)) => {
                let mut checksum_buf = [checksum_out; 1];
                match self.spi.transfer(&mut checksum_buf) {
                    Ok(_) => Ok(Some(checksum_buf[0])),
                    Err(e) => Err(e),
                }
            }
            (Err(e), _) =>
                Err(e),
        };
        let _ = self.nss.set_high();

        result
    }
}
