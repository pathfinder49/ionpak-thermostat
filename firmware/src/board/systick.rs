use cortex_m_rt::exception;
use cortex_m::peripheral::{SYST, syst::SystClkSource};

const SYSTICK_RATE: u32 = 1000;

static mut TIME: u64 = 0;

pub fn init() {
    unsafe { TIME = 0 };
    
    #[allow(mutable_transmutes)]
    let syst: &mut SYST = unsafe { core::mem::transmute(&*SYST::ptr()) };
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(100 * SYST::get_ticks_per_10ms() / SYSTICK_RATE);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
}

#[exception]
unsafe fn SysTick() {
    TIME += u64::from(1000 / SYSTICK_RATE);
}

pub fn get_time() -> u64 {
    unsafe { TIME }
}
