mod regs;
mod checksum;
pub use checksum::ChecksumMode;
mod adc;
pub use adc::*;

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
