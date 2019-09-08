use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::{SYST, syst::SystClkSource};
use cortex_m_rt::exception;
use bare_metal::CriticalSection;

static mut TIME: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));
/// In HZ
const RATE: u32 = 10;
/// Period between to interrupts in ns
const INTERVAL: u64 = 1_000_000 / RATE as u64;

fn syst() -> &'static mut SYST {
    #[allow(mutable_transmutes)]
    unsafe { core::mem::transmute(&*SYST::ptr()) }
}

pub fn init(_cs: &CriticalSection) {
    let syst = syst();
    // syst.set_clock_source(SystClkSource::Core);
    syst.set_clock_source(SystClkSource::External);
    syst.set_reload(100 * SYST::get_ticks_per_10ms() / RATE);
    syst.clear_current();
    syst.enable_interrupt();
    syst.enable_counter();
}

#[exception]
unsafe fn SysTick() {
    cortex_m::interrupt::free(|cs| {
        TIME.borrow(cs).replace_with(|time| *time + INTERVAL);
    });
}

pub fn get_time() -> u64 {
    let base = cortex_m::interrupt::free(|cs| {
        *unsafe { &mut TIME }.borrow(cs).borrow()
    });
    let syst_current = u64::from(SYST::get_current());
    let syst_reload = u64::from(SYST::get_reload());
    let precise = INTERVAL - (INTERVAL * syst_current / syst_reload);
    base + u64::from(precise)
}
