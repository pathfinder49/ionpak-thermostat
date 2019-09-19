use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::spi::Transfer;
use super::checksum::{ChecksumMode, Checksum};
use super::AdcError;
use super::{
    regs, regs::RegisterData,
    Input, RefSource, PostFilter, DigitalFilterOrder,
};

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

    /// `0x00DX` for AD7172-2
    pub fn identify(&mut self) -> Result<u16, AdcError<SPI::Error>> {
        self.read_reg(&regs::Id)
            .map(|id| id.id())
    }

    pub fn set_checksum_mode(&mut self, mode: ChecksumMode) -> Result<(), AdcError<SPI::Error>> {
        // Cannot use update_reg() here because checksum_mode is
        // updated between read_reg() and write_reg().
        let mut ifmode = self.read_reg(&regs::IfMode)?;
        ifmode.set_crc(mode);
        self.checksum_mode = mode;
        self.write_reg(&regs::IfMode, &mut ifmode)?;
        Ok(())
    }

    pub fn set_sync_enable(&mut self, enable: bool) -> Result<(), AdcError<SPI::Error>> {
        self.update_reg(&regs::GpioCon, |data| {
            data.set_sync_en(enable);
        })
    }

    pub fn setup_channel(
        &mut self, index: u8, in_pos: Input, in_neg: Input
    ) -> Result<(), AdcError<SPI::Error>> {
        self.update_reg(&regs::SetupCon { index }, |data| {
            data.set_bipolar(false);
            data.set_refbuf_pos(true);
            data.set_refbuf_neg(true);
            data.set_ainbuf_pos(true);
            data.set_ainbuf_neg(true);
            data.set_ref_sel(RefSource::External);
        })?;
        self.update_reg(&regs::FiltCon { index }, |data| {
            data.set_enh_filt_en(true);
            data.set_enh_filt(PostFilter::F16SPS);
            data.set_order(DigitalFilterOrder::Sinc5Sinc1);
        })?;
        // let mut offset = <regs::Offset as regs::Register>::Data::empty();
        // offset.set_offset(0);
        // self.write_reg(&regs::Offset { index }, &mut offset);
        self.update_reg(&regs::Channel { index }, |data| {
            data.set_setup(index);
            data.set_enabled(true);
            data.set_a_in_pos(in_pos);
            data.set_a_in_neg(in_neg);
        })?;
        Ok(())
    }

    pub fn get_postfilter(&mut self, index: u8) -> Result<Option<PostFilter>, AdcError<SPI::Error>> {
        self.read_reg(&regs::FiltCon { index })
            .map(|data| {
                if data.enh_filt_en() {
                    Some(data.enh_filt())
                } else {
                    None
                }
            })
    }

    pub fn set_postfilter(&mut self, index: u8, filter: Option<PostFilter>) -> Result<(), AdcError<SPI::Error>> {
        self.update_reg(&regs::FiltCon { index }, |data| {
            match filter {
                None => data.set_enh_filt_en(false),
                Some(filter) => {
                    data.set_enh_filt_en(true);
                    data.set_enh_filt(filter);
                }
            }
        })
    }

    /// Returns the channel the data is from
    pub fn data_ready(&mut self) -> Result<Option<u8>, AdcError<SPI::Error>> {
        self.read_reg(&regs::Status)
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
        self.read_reg(&regs::Data)
            .map(|data| data.data())
    }

    fn read_reg<R: regs::Register>(&mut self, reg: &R) -> Result<R::Data, AdcError<SPI::Error>> {
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

    fn write_reg<R: regs::Register>(&mut self, reg: &R, reg_data: &mut R::Data) -> Result<(), AdcError<SPI::Error>> {
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
        R: regs::Register,
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
