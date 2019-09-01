use core::mem::transmute;
use core::slice::from_raw_parts_mut;
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub trait Gpio where Self: Sized {
    fn into_output(self) -> GpioOutput<Self>;
    fn into_input(self) -> GpioInput<Self>;
}

pub struct GpioInput<PIN>(PIN);
pub struct GpioOutput<PIN>(PIN);

macro_rules! def_gpio {
    ($PORT: tt, $PIN: tt, $idx: expr) => (
        impl Gpio for $PIN {
            fn into_output(self) -> GpioOutput<Self> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                gpio.dir.modify(|_, w| w.dir().bits(1 << $idx));
                gpio.den.modify(|_, w| w.den().bits(1 << $idx));
                GpioOutput(self)
            }
            fn into_input(self) -> GpioInput<Self> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                gpio.dir.modify(|r, w| w.dir().bits(r.dir().bits() & !(1 << $idx)));
                gpio.den.modify(|_, w| w.den().bits(1 << $idx));
                GpioInput(self)
            }
        }

        impl InputPin for GpioInput<$PIN> {
            type Error = ();
            fn is_high(&self) -> Result<bool, Self::Error> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                Ok(gpio.data.read().data().bits() & (1 << $idx) == (1 << $idx))
            }
            fn is_low(&self) -> Result<bool, Self::Error> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                Ok(gpio.data.read().data().bits() & (1 << $idx) != (1 << $idx))
            }
        }

        impl OutputPin for GpioOutput<$PIN> {
            type Error = ();
            fn set_low(&mut self) -> Result<(), Self::Error> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                let data = masked_data(unsafe { transmute(&gpio.data) }, (1 << $idx));
                *data = 0;
                Ok(())
            }
            fn set_high(&mut self) -> Result<(), Self::Error> {
                let gpio = unsafe { &*tm4c129x::$PORT::ptr() };
                let data = masked_data(unsafe { transmute(&gpio.data) }, (1 << $idx));
                *data = 1 << $idx;
                Ok(())
            }
        }
    )
}

pub struct PB4;
def_gpio!(GPIO_PORTB_AHB, PB4, 4);
pub struct PB5;
def_gpio!(GPIO_PORTB_AHB, PB5, 5);
pub struct PE4;
def_gpio!(GPIO_PORTE_AHB, PE4, 4);
pub struct PE5;
def_gpio!(GPIO_PORTE_AHB, PE5, 5);

/// Setting of GPIO pins is optimized by address masking
fn masked_data<'a>(data: *mut u32, bits: u8) -> &'a mut u32 {
    let data = unsafe { from_raw_parts_mut(data, 0x400) };
    &mut data[usize::from(bits)]
}
