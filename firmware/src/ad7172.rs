use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::spi::Transfer;

#[allow(unused)]
#[repr(u8)]
pub enum Register {
    Status = 0x00,
    AdcMode = 0x01,
    IfMode = 0x02,
    RegCheck = 0x03,
    Data = 0x04,
    GpioCon = 0x06,
    Id = 0x07,
    Ch0 = 0x10,
    Ch1 = 0x11,
    Ch2 = 0x12,
    Ch3 = 0x13,
    SetupCon0 = 0x20,
    SetupCon1 = 0x21,
    SetupCon2 = 0x22,
    SetupCon3 = 0x23,
    FiltCon0 = 0x28,
    FiltCon1 = 0x29,
    FiltCon2 = 0x2A,
    FiltCon3 = 0x2B,
    Offset0 = 0x30,
    Offset1 = 0x31,
    Offset2 = 0x32,
    Offset3 = 0x33,
    Gain0 = 0x38,
    Gain1 = 0x39,
    Gain2 = 0x3A,
    Gain3 = 0x3B,
}

pub struct Adc<SPI: Transfer<u8>, NSS: OutputPin> {
    spi: SPI,
    nss: NSS,
}

impl<SPI: Transfer<u8>, NSS: OutputPin> Adc<SPI, NSS> {
    pub fn new(spi: SPI, nss: NSS) -> Result<Self, SPI::Error> {
        let mut adc = Adc { spi, nss};

        let mut buf = [0, 0, 0];
        adc.write_reg(Register::AdcMode, &mut buf)?;
        let mut buf = [0, 1, 0];
        adc.write_reg(Register::IfMode, &mut buf)?;
        let mut buf = [0, 0, 0];
        adc.write_reg(Register::GpioCon, &mut buf)?;

        Ok(adc)
    }

    /// Returns the channel the data is from
    pub fn data_ready(&mut self) -> Option<u8> {
        let mut buf = [0u8; 2];
        match self.read_reg(Register::Status, &mut buf) {
            Err(_) => None,
            Ok(()) => {
                if buf[1] & 0x80 == 0 {
                    None
                } else {
                    Some(buf[1] & 0x3)
                }
            }
        }
    }

    /// Get data
    pub fn read_data(&mut self) -> Result<u32, SPI::Error> {
        let mut buf = [0u8; 4];
        self.read_reg(Register::Data, &mut buf)?;
        let result =
            (u32::from(buf[1]) << 16) |
            (u32::from(buf[2]) << 8) |
            u32::from(buf[3]);
        Ok(result)
    }

    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], SPI::Error> {
        let _ = self.nss.set_low();
        let result = self.spi.transfer(words);
        let _ = self.nss.set_high();
        result
    }

    fn read_reg(&mut self, reg: Register, buffer: &'_ mut [u8]) -> Result<(), SPI::Error> {
        buffer[0] = reg as u8;
        self.transfer(buffer)?;
        Ok(())
    }

    fn write_reg(&mut self, reg: Register, buffer: &'_ mut [u8]) -> Result<(), SPI::Error> {
        buffer[0] = 0x40 | (reg as u8);
        self.transfer(buffer)?;
        Ok(())
    }
}
