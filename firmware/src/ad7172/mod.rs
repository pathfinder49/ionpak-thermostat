mod regs;
mod checksum;
pub use checksum::ChecksumMode;
mod adc;
pub use adc::*;

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
    Invalid = 0b11111,
}

impl From<u8> for Input {
    fn from(x: u8) -> Self {
        match x {
            0 => Input::Ain0,
            1 => Input::Ain1,
            2 => Input::Ain2,
            3 => Input::Ain3,
            4 => Input::Ain4,
            17 => Input::TemperaturePos,
            18 => Input::TemperatureNeg,
            19 => Input::AnalogSupplyPos,
            20 => Input::AnalogSupplyNeg,
            21 => Input::RefPos,
            22 => Input::RefNeg,
            _ => Input::Invalid,
        }
    }
}

/// Reference source for ADC conversion
#[repr(u8)]
pub enum RefSource {
    /// External reference
    External = 0b00,
    /// Internal 2.5V reference
    Internal = 0b10,
    /// AVDD1 − AVSS
    Avdd1MinusAvss = 0b11,
    Invalid = 0b01,
}

impl From<u8> for RefSource {
    fn from(x: u8) -> Self {
        match x {
            0 => RefSource::External,
            1 => RefSource::Internal,
            2 => RefSource::Avdd1MinusAvss,
            _ => RefSource::Invalid,
        }
    }
}

#[repr(u8)]
pub enum PostFilter {
    F27SPS = 0b010,
    F21SPS = 0b011,
    F20SPS = 0b101,
    F16SPS = 0b110,
    Invalid = 0b111,
}

impl From<u8> for PostFilter {
    fn from(x: u8) -> Self {
        match x {
            0b010 => PostFilter::F27SPS,
            0b011 => PostFilter::F21SPS,
            0b101 => PostFilter::F20SPS,
            0b110 => PostFilter::F16SPS,
            _ => PostFilter::Invalid,
        }
    }
}

#[repr(u8)]
pub enum DigitalFilterOrder {
    Sinc5Sinc1 = 0b00,
    Sinc3 = 0b11,
    Invalid = 0b10,
}

impl From<u8> for DigitalFilterOrder {
    fn from(x: u8) -> Self {
        match x {
            0b00 => DigitalFilterOrder::Sinc5Sinc1,
            0b11 => DigitalFilterOrder::Sinc3,
            _ => DigitalFilterOrder::Invalid,
        }
    }
}
