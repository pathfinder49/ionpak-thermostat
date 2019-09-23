use crate::board::pwm::{self, PwmChannel, PwmPeripheral};

#[derive(Clone, Copy, Debug)]
pub enum TecPin {
    ISet,
    MaxIPos,
    MaxINeg,
    MaxV,
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
        let (t2ccp0, t2ccp1) = tm4c129x::TIMER2::split();
        let (t3ccp0, t3ccp1) = tm4c129x::TIMER3::split();
        Tec {
            max_i_pos: t2ccp0,
            max_i_neg: t2ccp1,
            i_set: t3ccp0,
            max_v: t3ccp1,
        }
    }
}

impl Tec<pwm::T4CCP0, pwm::T4CCP1, pwm::T5CCP0, pwm::T5CCP1> {
    pub fn tec1() -> Self {
        let (t4ccp0, t4ccp1) = tm4c129x::TIMER4::split();
        let (t5ccp0, t5ccp1) = tm4c129x::TIMER5::split();
        Tec {
            max_i_pos: t4ccp0,
            max_i_neg: t4ccp1,
            i_set: t5ccp0,
            max_v: t5ccp1,
        }
    }
}


impl<MaxIPos: PwmChannel, MaxINeg: PwmChannel, ISet: PwmChannel, MaxV: PwmChannel> Tec<MaxIPos, MaxINeg, ISet, MaxV> {
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
