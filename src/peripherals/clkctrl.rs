use crate::memory::MemoryMapped;

use super::{Ccp, ClockSource};

const CLKCTRL_MCLKCTRLA: usize = 0x00;
const CLKCTRL_MCLKCTRLB: usize = 0x01;
const CLKCTRL_MCLKLOCK: usize = 0x02;
const CLKCTRL_MCLKSTATUS: usize = 0x03;
const _CLKCTRL_OSC20MCTRLA: usize = 0x10;
const _CLKCTRL_OSC20MCALIBA: usize = 0x11;
const _CLKCTRL_OSC20MCALIBB: usize = 0x12;
const _CLKCTRL_OSC32KCTRLA: usize = 0x18;
const CLKCTRL_XOSC32KCTRLA: usize = 0x1C;

pub struct Clkctrl {
    regs: [u8; 0x1D],
    clock_period: u64,
    ccp: bool,
}

impl Clkctrl {
    pub fn new() -> Self {
        Clkctrl {
            regs: [0; 0x1D],
            clock_period: 300,
            ccp: false,
        }
    }

    fn is_locked(&self) -> bool {
        self.regs[CLKCTRL_MCLKLOCK] & 0x1 == 1
    }

    fn update_clock(&mut self) {
        let period = match self.regs[CLKCTRL_MCLKCTRLA] & 0x3 {
            0 => 50,
            1 => 30518,
            _ => return,
        };

        if self.regs[CLKCTRL_MCLKCTRLB] & 0x1 == 0 {
            self.clock_period = period;
            println!(
                "[INFO] CLK_MAIN changed to {:.3} MHz.",
                1e3 / (f64::from(self.clock_period as u32))
            );
            return;
        }

        let pdiv = match (self.regs[CLKCTRL_MCLKCTRLB] >> 1) & 0xF {
            0 => 2,
            1 => 4,
            2 => 8,
            3 => 16,
            4 => 32,
            5 => 64,
            8 => 6,
            9 => 10,
            10 => 12,
            11 => 24,
            12 => 48,
            _ => {
                println!("[WARNING] Invalid main prescaler specified. Write to MCLKCTRLB will be ignored.");
                return;
            }
        };

        self.clock_period = period * pdiv;
        println!(
            "[INFO] CLK_MAIN changed to {:.3} MHz.",
            1e3 / (f64::from(self.clock_period as u32))
        );
    }
}

impl MemoryMapped for Clkctrl {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        if let CLKCTRL_MCLKSTATUS..=CLKCTRL_XOSC32KCTRLA = address {
            println!("[WARNING] CLKCTRL MCLKSTATUS..XOSC32KXTRLA registers are not implemented in this emulator. Reads will return 0.")
        }
        (self.regs[address], 0)
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            CLKCTRL_MCLKCTRLA => {
                if self.ccp & !self.is_locked() {
                    self.regs[CLKCTRL_MCLKCTRLA] = value & 0x3;
                    match value & 0x3 {
                        0 => self.update_clock(),
                        1 => self.update_clock(),
                        2 => println!(
                            "[WARNING] XOSC32K is not supported. This write will be ignored."
                        ),
                        3 => {
                            self.update_clock();
                            println!("[WARNING] EXTCLK is set to 8 MHz in this emulator which may not be consistent with hardware.")
                        }
                        _ => {}
                    }
                    if value & 0x80 != 0 {
                        println!("[WARNING] CLKOUT feature is not implemented in this emulator. This bit will be ignored.");
                    }
                }
            }
            CLKCTRL_MCLKCTRLB => {
                if self.ccp & !self.is_locked() {
                    self.regs[CLKCTRL_MCLKCTRLB] = value & 0x1F;
                    self.update_clock();
                }
            }
            CLKCTRL_MCLKLOCK => {
                if self.ccp & !self.is_locked() {
                    self.regs[CLKCTRL_MCLKLOCK] = value & 0x1;
                }
            }
            CLKCTRL_MCLKSTATUS..=CLKCTRL_XOSC32KCTRLA => {
                println!("[WARNING] CLKCTRL MCLKSTATUS..XOSC32KXTRLA registers are not implemented in this emulator. Writes will be ignored.");
            }
            _ => {}
        }
        0
    }
}

impl Ccp for Clkctrl {
    fn ccp(&mut self, enabled: bool) {
        self.ccp = enabled;
    }
}

impl ClockSource for Clkctrl {
    fn clock_period(&self) -> u64 {
        self.clock_period
    }
}
