use core::fmt;
use crate::board::pwm::{self, PwmChannel, PwmPeripheral};

#[derive(Clone, Copy, Debug)]
pub enum TecPin {
    ISet,
    MaxIPos,
    MaxINeg,
    MaxV,
}

impl TecPin {
    pub const VALID_VALUES: &'static [TecPin] = &[
        TecPin::ISet,
        TecPin::MaxIPos,
        TecPin::MaxINeg,
        TecPin::MaxV,
    ];
}

impl fmt::Display for TecPin {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            TecPin::ISet =>
                "i_set".fmt(fmt),
            TecPin::MaxIPos =>
                "max_i_pos".fmt(fmt),
            TecPin::MaxINeg =>
                "max_i_neg".fmt(fmt),
            TecPin::MaxV =>
                "max_v".fmt(fmt),
        }
    }
}

/// Thermo-Electric Cooling device controlled through four PWM
/// channels
pub struct Tec<MaxIPos: PwmChannel, MaxINeg: PwmChannel, ISet: PwmChannel, MaxV: PwmChannel> {
    max_i_pos: MaxIPos,
    max_i_neg: MaxINeg,
    i_set: ISet,
    max_v: MaxV,
}

impl Tec<pwm::T2CCP0, pwm::T2CCP1, pwm::T3CCP0, pwm::T3CCP1> {
    pub fn tec0() -> Self {
        let (max_i_pos, max_i_neg) = tm4c129x::TIMER2::split();
        let (i_set, max_v) = tm4c129x::TIMER3::split();
        Tec { max_i_pos, max_i_neg, i_set, max_v }
    }
}

impl Tec<pwm::T4CCP0, pwm::T4CCP1, pwm::T5CCP0, pwm::T5CCP1> {
    pub fn tec1() -> Self {
        let (max_i_pos, max_i_neg) = tm4c129x::TIMER4::split();
        let (i_set, max_v) = tm4c129x::TIMER5::split();
        Tec { max_i_pos, max_i_neg, i_set, max_v }
    }
}


impl<MaxIPos: PwmChannel, MaxINeg: PwmChannel, ISet: PwmChannel, MaxV: PwmChannel> Tec<MaxIPos, MaxINeg, ISet, MaxV> {
    pub fn get(&mut self, pin: TecPin) -> (u16, u16) {
        match pin {
            TecPin::MaxIPos =>
                self.max_i_pos.get(),
            TecPin::MaxINeg =>
                self.max_i_neg.get(),
            TecPin::ISet =>
                self.i_set.get(),
            TecPin::MaxV =>
                self.max_v.get(),
        }
    }

    pub fn set(&mut self, pin: TecPin, width: u16, total: u16) {
        match pin {
            TecPin::MaxIPos =>
                self.max_i_pos.set(width, total),
            TecPin::MaxINeg =>
                self.max_i_neg.set(width, total),
            TecPin::ISet =>
                self.i_set.set(width, total),
            TecPin::MaxV =>
                self.max_v.set(width, total),
        }
    }
}
