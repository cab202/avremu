pub mod port;
pub mod spi;
pub mod stdio;
pub mod cpuint;
pub mod tcb;
pub mod tca;
pub mod adc;
pub mod usart;

pub trait InterruptSource {
    fn interrupt(&self, mask: u8) -> bool {
        // This function should return the bitwise and of the 
        // corresponding intflag and intctrl registers of the peripheral
        false
    }
}

pub trait Clocked {
    fn tick(&mut self, time: usize) {

    }
}

