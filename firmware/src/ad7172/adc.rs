use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::spi::Transfer;
use super::checksum::{ChecksumMode, Checksum};
use super::AdcError;
use super::regs::*;

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

    pub fn read_reg<R: Register>(&mut self, reg: &R) -> Result<R::Data, AdcError<SPI::Error>> {
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

    pub fn write_reg<R: Register>(&mut self, reg: &R, reg_data: &mut R::Data) -> Result<(), AdcError<SPI::Error>> {
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

    pub fn update_reg<R, F, A>(&mut self, reg: &R, f: F) -> Result<A, AdcError<SPI::Error>>
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
