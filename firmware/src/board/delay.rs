use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use tm4c129x::CorePeripherals;

/// precision internal oscillator
const PIOSC: u32 = 16_000_000;

pub struct Delay {
    syst: SYST,
}

impl Delay {
    /// unsafe: must only be used once to avoid concurrent use of systick
    pub unsafe fn new() -> Self {
        let mut syst = CorePeripherals::steal().SYST;
        // PIOSC
        syst.set_clock_source(SystClkSource::External);
        syst.disable_interrupt();
        Delay { syst }
    }
}

impl DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }
}

impl DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(u32::from(ms));
    }
}

impl DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(u32::from(ms));
    }
}

impl DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        // The SysTick Reload Value register supports values between 1 and 0x00FFFFFF.
        const MAX_RVR: u32 = 0x00FF_FFFF;

        let mut total_rvr = us * (PIOSC / 1_000_000);

        while total_rvr != 0 {
            let current_rvr = total_rvr.min(MAX_RVR);

            self.syst.set_reload(current_rvr);
            self.syst.clear_current();
            self.syst.enable_counter();

            // Update the tracking variable while we are waiting...
            total_rvr -= current_rvr;

            while !self.syst.has_wrapped() {}

            self.syst.disable_counter();
        }
    }
}

impl DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        self.delay_us(u32::from(us))
    }
}

impl DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        self.delay_us(u32::from(us))
    }
}
