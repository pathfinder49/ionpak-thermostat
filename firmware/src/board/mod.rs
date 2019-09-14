use cortex_m;
use tm4c129x;

pub mod gpio;
pub mod softspi;
pub mod systick;


const UART_DIV: u32 = (((/*sysclk*/120_000_000 * 8) / /*baud*/115200) + 1) / 2;

pub fn init() {
    cortex_m::interrupt::free(|cs| {
        let sysctl = unsafe { &*tm4c129x::SYSCTL::ptr() };

        // Set up main oscillator
        sysctl.moscctl.write(|w| w.noxtal().bit(false));
        sysctl.moscctl.modify(|_, w| w.pwrdn().bit(false).oscrng().bit(true));

        // Prepare flash for the high-freq clk
        sysctl.memtim0.write(|w| unsafe { w.bits(0x01950195u32) });
        sysctl.rsclkcfg.write(|w| unsafe { w.bits(0x80000000u32) });

        // Set up PLL with fVCO=480 MHz
        sysctl.pllfreq1.write(|w| w.q().bits(0).n().bits(4));
        sysctl.pllfreq0.write(|w| w.mint().bits(96).pllpwr().bit(true));
        sysctl.rsclkcfg.modify(|_, w| w.pllsrc().mosc().newfreq().bit(true));
        while !sysctl.pllstat.read().lock().bit() {}

        // Switch to PLL (sysclk=120MHz)
        sysctl.rsclkcfg.write(|w| unsafe { w.bits(0b1_0_0_1_0011_0000_0000000000_0000000011) });

        // Bring up GPIO ports A, B, D, E, F, G, K, L, M, P, Q
        sysctl.rcgcgpio.modify(|_, w| {
            w.r0().bit(true)
             .r1().bit(true)
             .r3().bit(true)
             .r4().bit(true)
             .r5().bit(true)
             .r6().bit(true)
             .r9().bit(true)
             .r10().bit(true)
             .r11().bit(true)
             .r13().bit(true)
             .r14().bit(true)
        });
        while !sysctl.prgpio.read().r0().bit() {}
        while !sysctl.prgpio.read().r1().bit() {}
        while !sysctl.prgpio.read().r3().bit() {}
        while !sysctl.prgpio.read().r4().bit() {}
        while !sysctl.prgpio.read().r5().bit() {}
        while !sysctl.prgpio.read().r6().bit() {}
        while !sysctl.prgpio.read().r9().bit() {}
        while !sysctl.prgpio.read().r10().bit() {}
        while !sysctl.prgpio.read().r11().bit() {}
        while !sysctl.prgpio.read().r13().bit() {}
        while !sysctl.prgpio.read().r14().bit() {}

        // Set up UART0
        let gpio_a = unsafe { &*tm4c129x::GPIO_PORTA_AHB::ptr() };
        gpio_a.dir.write(|w| w.dir().bits(0b11));
        gpio_a.den.write(|w| w.den().bits(0b11));
        gpio_a.afsel.write(|w| w.afsel().bits(0b11));
        gpio_a.pctl.write(|w| unsafe { w.pmc0().bits(1).pmc1().bits(1) });

        sysctl.rcgcuart.modify(|_, w| w.r0().bit(true));
        while !sysctl.pruart.read().r0().bit() {}

        let uart_0 = unsafe { &*tm4c129x::UART0::ptr() };
        uart_0.cc.write(|w| w.cs().sysclk());
        uart_0.ibrd.write(|w| w.divint().bits((UART_DIV / 64) as u16));
        uart_0.fbrd.write(|w| w.divfrac().bits((UART_DIV % 64) as u8));
        uart_0.lcrh.write(|w| w.wlen()._8().fen().bit(true));
        uart_0.ctl.write(|w| w.rxe().bit(true).txe().bit(true).uarten().bit(true));

        // Set up PWMs
        let gpio_m = unsafe { &*tm4c129x::GPIO_PORTM::ptr() };
        // Output
        gpio_m.dir.write(|w| w.dir().bits(0xff));
        // Enable
        gpio_m.den.write(|w| w.den().bits(0xff));
        // Alternate function
        gpio_m.afsel.write(|w| w.afsel().bits(0xff));
        // Function: Timer PWM
        gpio_m.pctl.write(|w| unsafe {
            w
                // t2ccp0
                .pmc0().bits(3)
                // t2ccp1
                .pmc1().bits(3)
                // t3ccp0
                .pmc2().bits(3)
                // t3ccp1
                .pmc3().bits(3)
                // t4ccp0
                .pmc4().bits(3)
                // t4ccp1
                .pmc5().bits(3)
                // t5ccp0
                .pmc6().bits(3)
                // t5ccp1
                .pmc7().bits(3)
        });

        // Enable timers
        sysctl.rcgctimer.write(|w| w
            .r2().set_bit()
            .r3().set_bit()
            .r4().set_bit()
            .r5().set_bit()
        );
        // Reset timers
        sysctl.srtimer.write(|w| w
            .r2().set_bit()
            .r3().set_bit()
            .r4().set_bit()
            .r5().set_bit()
        );
        sysctl.srtimer.write(|w| w
            .r2().clear_bit()
            .r3().clear_bit()
            .r4().clear_bit()
            .r5().clear_bit()
        );
        fn timers_ready(sysctl: &tm4c129x::sysctl::RegisterBlock) -> bool {
            let prtimer = sysctl.prtimer.read();
            prtimer.r2().bit() &&
                prtimer.r3().bit() &&
                prtimer.r4().bit() &&
                prtimer.r5().bit()
        }
        while !timers_ready(sysctl) {}

        // Manual: 13.4.5 PWM Mode
        macro_rules! setup_timer_pwm {
            ($T: tt) => (
                let timer = unsafe { &*tm4c129x::$T::ptr() };
                timer.cfg.write(|w| unsafe { w.bits(4) });
                timer.tamr.modify(|_, w| unsafe {
                    w
                        .taams().bit(true)
                        .tacmr().bit(false)
                        .tamr().bits(2)
                });
                timer.tbmr.modify(|_, w| unsafe {
                    w
                        .tbams().bit(true)
                        .tbcmr().bit(false)
                        .tbmr().bits(2)
                });
                timer.ctl.modify(|_, w| {
                    w
                        .tapwml().bit(false)
                        .tbpwml().bit(false)
                });
                // no prescaler
                // no interrupts
                timer.tailr.write(|w| unsafe { w.bits(0xFFFF) });
                timer.tbilr.write(|w| unsafe { w.bits(0xFFFF) });
                timer.tamatchr.write(|w| unsafe { w.bits(0x8000) });
                timer.tbmatchr.write(|w| unsafe { w.bits(0x8000) });
                timer.ctl.modify(|_, w| {
                    w
                        .taen().bit(true)
                        .tben().bit(true)
                });
            )
        }
        setup_timer_pwm!(TIMER2);
        setup_timer_pwm!(TIMER3);
        setup_timer_pwm!(TIMER4);
        setup_timer_pwm!(TIMER5);

        systick::init(cs);
    });
}

pub fn set_timer_pwm(matchr: u32, ilr: u32) {
    macro_rules! set_timer_pwm {
        ($T: tt) => (
            let timer = unsafe { &*tm4c129x::$T::ptr() };
            timer.tamatchr.write(|w| unsafe { w.bits(matchr) });
            timer.tbmatchr.write(|w| unsafe { w.bits(matchr) });
            timer.tailr.write(|w| unsafe { w.bits(ilr) });
            timer.tbilr.write(|w| unsafe { w.bits(ilr) });
        )
    }
    set_timer_pwm!(TIMER2);
    set_timer_pwm!(TIMER3);
    set_timer_pwm!(TIMER4);
    set_timer_pwm!(TIMER5);
}

pub fn get_mac_address() -> [u8; 6] {
    let (userreg0, userreg1) = cortex_m::interrupt::free(|_cs| {
        let flashctl = unsafe { &*tm4c129x::FLASH_CTRL::ptr() };
        (flashctl.userreg0.read().bits(),
         flashctl.userreg1.read().bits())
    });
    [userreg0 as u8, (userreg0 >> 8) as u8, (userreg0 >> 16) as u8,
     userreg1 as u8, (userreg1 >> 8) as u8, (userreg1 >> 16) as u8]
}
