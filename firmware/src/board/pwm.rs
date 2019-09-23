use tm4c129x::{
    TIMER2, TIMER3, TIMER4, TIMER5,
};

pub struct T2CCP0;
pub struct T2CCP1;
pub struct T3CCP0;
pub struct T3CCP1;
pub struct T4CCP0;
pub struct T4CCP1;
pub struct T5CCP0;
pub struct T5CCP1;

pub trait PwmPeripheral {
    type ChannelA: PwmChannel;
    type ChannelB: PwmChannel;
    fn split() -> (Self::ChannelA, Self::ChannelB);
}

macro_rules! pwm_peripheral {
    ($TIMER: ty, $A: tt, $B: tt) => {
        impl PwmPeripheral for $TIMER {
            type ChannelA = $A;
            type ChannelB = $B;
            fn split() -> (Self::ChannelA, Self::ChannelB) {
                let regs = unsafe { &*Self::ptr() };
                regs.cfg.write(|w| unsafe { w.bits(4) });

                let mut a = $A;
                a.configure();
                let mut b = $B;
                b.configure();
                (a, b)
            }
        }
    };
}

pwm_peripheral!(TIMER2, T2CCP0, T2CCP1);
pwm_peripheral!(TIMER3, T3CCP0, T3CCP1);
pwm_peripheral!(TIMER4, T4CCP0, T4CCP1);
pwm_peripheral!(TIMER5, T5CCP0, T5CCP1);


pub trait PwmChannel {
    fn configure(&mut self);
    fn get(&mut self) -> (u16, u16);
    fn set(&mut self, width: u16, total: u16);
}

macro_rules! pwm_channel_a {
    ($CHANNEL: ty, $TIMER: tt) => {
        impl PwmChannel for $CHANNEL {
            fn configure(&mut self) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                timer.tamr.modify(|_, w| unsafe {
                    w
                        .taams().bit(true)
                        .tacmr().bit(false)
                        .tamr().bits(2)
                });
                timer.ctl.modify(|_, w| {
                    w
                        .tapwml().bit(false)
                });
                // no prescaler
                // no interrupts
                timer.tailr.write(|w| unsafe { w.bits(0xFFFF) });
                timer.tamatchr.write(|w| unsafe { w.bits(0x0) });
                timer.ctl.modify(|_, w| {
                    w
                        .taen().bit(true)
                });
            }

            fn get(&mut self) -> (u16, u16) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                (timer.tamatchr.read().bits() as u16,
                 timer.tailr.read().bits() as u16)
            }

            fn set(&mut self, width: u16, total: u16) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                timer.tamatchr.write(|w| unsafe { w.bits(width.into()) });
                timer.tailr.write(|w| unsafe { w.bits(total.into()) });
            }
        }
    };
}

macro_rules! pwm_channel_b {
    ($CHANNEL: ty, $TIMER: tt) => {
        impl PwmChannel for $CHANNEL {
            fn configure(&mut self) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                timer.tbmr.modify(|_, w| unsafe {
                    w
                        .tbams().bit(true)
                        .tbcmr().bit(false)
                        .tbmr().bits(2)
                });
                timer.ctl.modify(|_, w| {
                    w
                        .tbpwml().bit(false)
                });
                // no prescaler
                // no interrupts
                timer.tbilr.write(|w| unsafe { w.bits(0xFFFF) });
                timer.tbmatchr.write(|w| unsafe { w.bits(0x0) });
                timer.ctl.modify(|_, w| {
                    w
                        .tben().bit(true)
                });
            }

            fn get(&mut self) -> (u16, u16) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                (timer.tbmatchr.read().bits() as u16,
                 timer.tbilr.read().bits() as u16)
            }

            fn set(&mut self, width: u16, total: u16) {
                let timer = unsafe { &*tm4c129x::$TIMER::ptr() };
                timer.tbmatchr.write(|w| unsafe { w.bits(width.into()) });
                timer.tbilr.write(|w| unsafe { w.bits(total.into()) });
            }
        }
    };
}

pwm_channel_a!(T2CCP0, TIMER2);
pwm_channel_b!(T2CCP1, TIMER2);
pwm_channel_a!(T3CCP0, TIMER3);
pwm_channel_b!(T3CCP1, TIMER3);
pwm_channel_a!(T4CCP0, TIMER4);
pwm_channel_b!(T4CCP1, TIMER4);
pwm_channel_a!(T5CCP0, TIMER5);
pwm_channel_b!(T5CCP1, TIMER5);
