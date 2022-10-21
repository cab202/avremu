pub mod port;
pub mod spi;
pub mod stdio;
pub mod cpuint;
pub mod tcb;
pub mod tca;
pub mod adc;
pub mod usart;
pub mod clkctrl;
pub mod cpu;

pub trait InterruptSource {
    fn interrupt(&mut self, mask: u8) -> bool {
        // This function should return the bitwise and of the 
        // corresponding intflag and intctrl registers of the peripheral
        false
    }
}

pub trait Clocked {
    fn tick(&mut self, time: u64) {

    }
}

pub trait Ccp {
    fn ccp(&mut self, enabled: bool) {

    }
}

pub trait ClockSource {
    fn clock_period(&self) -> u64;
}

