pub mod adc;
pub mod clkctrl;
pub mod cpu;
pub mod cpuint;
pub mod port;
pub mod portmux;
pub mod spi;
pub mod stdio;
pub mod tca;
pub mod tcb;
pub mod usart;

pub trait InterruptSource {
    fn interrupt(&mut self, _mask: u8) -> bool {
        // This function should return the bitwise and of the
        // corresponding intflag and intctrl registers of the peripheral
        false
    }
}

pub trait Clocked {
    fn tick(&mut self, _time: u64) {}
}

pub trait Ccp {
    fn ccp(&mut self, _enabled: bool) {}
}

pub trait ClockSource {
    fn clock_period(&self) -> u64;
}
