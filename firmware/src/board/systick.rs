use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::{SYST, syst::SystClkSource};
use cortex_m_rt::exception;
use bare_metal::CriticalSection;

const SYSTICK_RATE: u32 = 250;

static mut TIME: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));

pub fn init(cs: &CriticalSection) {
    #[allow(mutable_transmutes)]
    let syst: &mut SYST = unsafe { core::mem::transmute(&*SYST::ptr()) };
    syst.set_clock_source(SystClkSource::External);
    syst.set_reload(100 * SYST::get_ticks_per_10ms() / SYSTICK_RATE);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
}

#[exception]
unsafe fn SysTick() {
    let interval = u64::from(1000 / SYSTICK_RATE);
    cortex_m::interrupt::free(|cs| {
        TIME.borrow(cs).replace_with(|time| *time + interval);
    });
}

pub fn get_time() -> u64 {
    cortex_m::interrupt::free(|cs| {
        *unsafe { &mut TIME }.borrow(cs).borrow()
    })
}
