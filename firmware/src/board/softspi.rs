use embedded_hal::spi::FullDuplex;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use nb::Error::WouldBlock;

/// Bit-banged SPI
pub struct SoftSpi<SCK, MOSI, MISO> {
    sck: SCK,
    mosi: MOSI,
    miso: MISO,
    state: State,
    input: Option<u8>,
}

#[derive(PartialEq)]
enum State {
    Idle,
    Transfer {
        clock_phase: bool,
        mask: u8,
        output: u8,
        input: u8,
    },
}

impl<SCK: OutputPin, MOSI: OutputPin, MISO: InputPin> SoftSpi<SCK, MOSI, MISO> {
    pub fn new(mut sck: SCK, mut mosi: MOSI, miso: MISO) -> Self {
        let _ = sck.set_high();
        let _ = mosi.set_low();
        SoftSpi {
            sck, mosi, miso,
            state: State::Idle,
            input: None,
        }
    }

    /// Call this at twice the data rate
    pub fn tick(&mut self) {
        match self.state {
            State::Idle => {}
            State::Transfer { clock_phase: false,
                              mask, output, input } => {
                if output & mask != 0 {
                    let _ = self.mosi.set_high();
                } else {
                    let _ = self.mosi.set_low();
                }
                let _ = self.sck.set_low();

                self.state = State::Transfer {
                    clock_phase: true,
                    mask, output, input,
                };
            }
            State::Transfer { clock_phase: true,
                              mask, output, mut input } => {
                let _ = self.sck.set_high();
                if self.miso.is_high().unwrap_or(false) {
                    input |= mask;
                }

                if mask != 1 {
                    self.state = State::Transfer {
                        clock_phase: false,
                        mask: mask >> 1,
                        output, input,
                    };
                } else {
                    self.input = Some(input);
                    self.state = State::Idle;
                }
            }
        }
    }
}

impl<SCK: OutputPin, MOSI: OutputPin, MISO: InputPin> FullDuplex<u8> for SoftSpi<SCK, MOSI, MISO> {
    type Error = ();

    fn read(&mut self) -> Result<u8, nb::Error<Self::Error>> {
        match self.input.take() {
            Some(input) =>
                Ok(input),
            None if self.state == State::Idle =>
                Err(nb::Error::Other(())),
            None =>
                Err(WouldBlock),
        }
    }

    fn send(&mut self, output: u8) -> Result<(), nb::Error<Self::Error>> {
        match self.state {
            State::Idle => {
                self.state = State::Transfer {
                    clock_phase: false,
                    mask: 0x80,
                    output,
                    input: 0,
                };
                Ok(())
            }
            _ => Err(WouldBlock)
        }
    }
}
