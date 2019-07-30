use cortex_m;
use tm4c129x;

mod gpio;


const LED1: u8 = 0x10; // PK4
const LED2: u8 = 0x40; // PK6

const FD_ADC: u8 = 0x01;  // PE0
const FV_ADC: u8 = 0x02;  // PE1
const FBI_ADC: u8 = 0x04; // PE2
const IC_ADC: u8 = 0x08;  // PE3
const FBV_ADC: u8 = 0x20; // PD5
const AV_ADC: u8 = 0x40;  // PD6

const FV_ERRN: u8 = 0x01;    // PL0
const FBV_ERRN: u8 = 0x02;   // PL1
const FBI_ERRN: u8 = 0x04;   // PL2
const AV_ERRN: u8 = 0x08;    // PL3
const AI_ERRN: u8 = 0x10;    // PL4
const ERR_LATCHN: u8 = 0x20; // PL5
const BTNN: u8 = 0x80;       // PL7
const ERR_RESN: u8 = 0x01;   // PQ0

pub const PWM_LOAD: u16 = (/*pwmclk*/120_000_000u32 / /*freq*/100_000) as u16;
const UART_DIV: u32 = (((/*sysclk*/120_000_000 * 8) / /*baud*/115200) + 1) / 2;


pub const AV_ADC_GAIN: f32 = 6.792703150912105;
pub const FV_ADC_GAIN: f32 = 501.83449105726623;
pub const FBI_ADC_GAIN: f32 = 1333.3333333333333;
pub const FBI_ADC_OFFSET: f32 = 96.0;
pub const FD_ADC_GAIN: f32 = 3111.1111111111104;
pub const FD_ADC_OFFSET: f32 = 96.0;
pub const FBV_ADC_GAIN: f32 = 49.13796058269066;
pub const FBV_PWM_GAIN: f32 = 0.07641071428571428;
pub const IC_ADC_GAIN_LOW: f32 = 1333333333333.3333;
pub const IC_ADC_GAIN_MED: f32 = 13201320132.0132;
pub const IC_ADC_GAIN_HIGH: f32 = 133320001.3332;
pub const IC_ADC_OFFSET: f32 = 96.0;

pub const FBI_R223: f32 = 200.0;
pub const FBI_R224: f32 = 39.0;
pub const FBI_R225: f32 = 22000.0;


pub fn reset_error() {
    cortex_m::interrupt::free(|_cs| {
        let gpio_q = unsafe { &*tm4c129x::GPIO_PORTQ::ptr() };
        gpio_q.data.modify(|r, w| w.data().bits(r.data().bits() & !ERR_RESN));
        gpio_q.data.modify(|r, w| w.data().bits(r.data().bits() | ERR_RESN));
    });
}

pub fn error_latched() -> bool {
    cortex_m::interrupt::free(|_cs| {
        let gpio_l = unsafe { &*tm4c129x::GPIO_PORTL::ptr() };
        gpio_l.data.read().bits() as u8 & ERR_LATCHN == 0
    })
}

pub fn process_errors() {
    let gpio_dat = cortex_m::interrupt::free(|_cs| {
        let gpio_l = unsafe { &*tm4c129x::GPIO_PORTL::ptr() };
        gpio_l.data.read().bits() as u8
    });
    if gpio_dat & FV_ERRN == 0 {
        println!("Filament overvolt");
    }
    if gpio_dat & FBV_ERRN == 0 {
        println!("Filament bias overvolt");
    }
    if gpio_dat & FBI_ERRN == 0 {
        println!("Filament bias overcurrent");
    }
    if gpio_dat & AV_ERRN == 0 {
        println!("Anode overvolt");
    }
    if gpio_dat & AI_ERRN == 0 {
        println!("Anode overcurrent");
    }
}

pub fn init() {
    cortex_m::interrupt::free(|_cs| {
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

        // Bring up GPIO ports A, D, E, F, G, K, L, M, P, Q
        sysctl.rcgcgpio.modify(|_, w| {
            w.r0().bit(true)
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
    });
}

pub fn start_adc() {
    cortex_m::interrupt::free(|_cs| {
        let sysctl = unsafe { &*tm4c129x::SYSCTL::ptr() };

        let gpio_d = unsafe { &*tm4c129x::GPIO_PORTD_AHB::ptr() };
        let gpio_e = unsafe { &*tm4c129x::GPIO_PORTE_AHB::ptr() };
        gpio_d.afsel.write(|w| w.afsel().bits(FBV_ADC|AV_ADC));
        gpio_d.amsel.write(|w| w.amsel().bits(FBV_ADC|AV_ADC));
        gpio_e.afsel.write(|w| w.afsel().bits(FD_ADC|FV_ADC|FBI_ADC|IC_ADC));
        gpio_e.amsel.write(|w| w.amsel().bits(FD_ADC|FV_ADC|FBI_ADC|IC_ADC));

        sysctl.rcgcadc.modify(|_, w| w.r0().bit(true));
        while !sysctl.pradc.read().r0().bit() {}

        let adc0 = unsafe { &*tm4c129x::ADC0::ptr() };
        // VCO 480 / 15 = 32MHz ADC clock
        adc0.cc.write(|w| w.cs().syspll().clkdiv().bits(15-1));
        adc0.im.write(|w| w.mask0().bit(true));
        adc0.emux.write(|w| w.em0().always());
        adc0.ssmux0.write(|w| {
            w.mux0().bits(0) // IC_ADC
             .mux1().bits(1) // FBI_ADC
             .mux2().bits(2) // FV_ADC
             .mux3().bits(3) // FD_ADC
             .mux4().bits(5) // AV_ADC
             .mux5().bits(6) // FBV_ADC
        });
        adc0.ssctl0.write(|w| w.ie5().bit(true).end5().bit(true));
        adc0.sstsh0.write(|w| {
            w.tsh0()._4()
             .tsh1()._4()
             .tsh2()._4()
             .tsh3()._4()
             .tsh4()._4()
             .tsh5()._4()
        });
        adc0.sac.write(|w| w.avg()._64x());
        adc0.ctl.write(|w| w.vref().bit(true));
        adc0.actss.write(|w| w.asen0().bit(true));

        let mut cp = unsafe { tm4c129x::CorePeripherals::steal() };
        cp.NVIC.enable(tm4c129x::Interrupt::ADC0SS0);
    });
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
