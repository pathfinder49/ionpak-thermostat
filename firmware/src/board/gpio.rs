use core::mem::transmute;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub trait Gpio where Self: Sized {
    fn into_output(self) -> GpioOutput<Self>;
    fn into_input(self) -> GpioInput<Self>;
}

pub struct GpioInput<PIN>(PIN);
pub struct GpioOutput<PIN>(PIN);

macro_rules! def_gpio {
    ($PORT: tt, $PIN: tt, $idx: expr) => (
        impl $PIN {
            fn data(&self) -> &u32 {
                let gpio = unsafe { tm4c129x::$PORT::ptr() };
                let data = unsafe { from_raw_parts(gpio as *const _ as *mut u32, 0x100) };
                &data[(1 << $idx) as usize]
            }

            fn data_mut(&mut self) -> &mut u32 {
                let gpio = unsafe { tm4c129x::$PORT::ptr() };
                let data = unsafe { from_raw_parts_mut(gpio as *const _ as *mut u32, 0x100) };
                &mut data[(1 << $idx) as usize]
            }
        }

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
                Ok(*self.0.data() != 0)
            }
            fn is_low(&self) -> Result<bool, Self::Error> {
                Ok(*self.0.data() == 0)
            }
        }

        impl OutputPin for GpioOutput<$PIN> {
            type Error = ();
            fn set_low(&mut self) -> Result<(), Self::Error> {
                *self.0.data_mut() = 0;
                Ok(())
            }
            fn set_high(&mut self) -> Result<(), Self::Error> {
                *self.0.data_mut() = 0xFF;
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
