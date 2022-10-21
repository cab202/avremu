use crate::memory::MemoryMapped;
use crate::peripherals::InterruptSource;
use crate::peripherals::Clocked;

const TCB_CTRLA:    usize = 0x00;
const TCB_CTRLB:    usize = 0x01;
const TCB_EVCTRL:   usize = 0x04;
const TCB_INTCTRL:  usize = 0x05;
const TCB_INTFLAGS: usize = 0x06;
const TCB_STATUS:   usize = 0x07;
const TCB_DBGCTRL:  usize = 0x08;
const TCB_TEMP:     usize = 0x09;
const TCB_CNTL:     usize = 0x0A;
const TCB_CNTH:     usize = 0x0B;
const TCB_CCMPL:    usize = 0x0C;
const TCB_CCMPH:    usize = 0x0D;

enum TCB_MODE {
    INT,
    TIMEOUT, 
    CAPT,
    FRQ,
    PW,
    FRQPW,
    SINGLE,
    PWM8
}

enum TCB_CLKSEL {
    DIV1,
    DIV2,
    TCA0,
    RESERVED,
    EVENT
}

pub struct Tcb {
    name: String, 
    regs: [u8; 0x0E],
    enabled: bool,
    clksel: TCB_CLKSEL,
    cntmode: TCB_MODE,
    tictoc: bool
}

impl Tcb {
    pub fn new(name: String) -> Self {
        Tcb {
            name,
            regs: [0; 0x0E],
            enabled: false,
            clksel: TCB_CLKSEL::DIV1,
            cntmode: TCB_MODE::INT,
            tictoc: false
        }
    }
}

impl MemoryMapped for Tcb {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            TCB_CTRLA..=TCB_TEMP => (self.regs[address], 0),
            TCB_CCMPL => { self.regs[TCB_TEMP] = self.regs[TCB_CCMPH]; (self.regs[TCB_CCMPL], 0) },
            TCB_CCMPH=>  (self.regs[TCB_TEMP], 0),
            TCB_CNTL => { self.regs[TCB_TEMP] = self.regs[TCB_CNTH]; (self.regs[TCB_CNTL], 0) },
            TCB_CNTH=> (self.regs[TCB_TEMP], 0),
            _ => (0, 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            TCB_CTRLA => {
                self.regs[TCB_CTRLA] = value;
                self.enabled = (value & 0x01) != 0;
                self.regs[TCB_STATUS] = if self.enabled {1} else {0};
                self.clksel = match (value >> 1) & 0x07 {
                    0x00 => TCB_CLKSEL::DIV1,
                    0x01 => TCB_CLKSEL::DIV2,
                    0x02 => {
                        println!("[WARNING] Clock selection TCA0 is not implemented for TCB in this emulator.");
                        TCB_CLKSEL::TCA0
                    },
                    0x07 => {
                        println!("[WARNING] Clock selection EVENT is not implemented for TCB in this emulator.");
                        TCB_CLKSEL::EVENT
                    },
                    _ => TCB_CLKSEL::RESERVED
                };
                if value & 0x70 != 0 {
                    println!("[WARNING] RUNSTDBY/CASCADE/SYNCUPD features are not implemented for TCB in this emulator. These bits will be ignored.");
                }
            },
            TCB_CTRLB => {
                self.regs[TCB_CTRLB] = value;
                self.cntmode = match value & 0x07 {
                    0x00 => TCB_MODE::INT,
                    0x01 => {
                        println!("[WARNING] TIMEOUT mode is not implemented for TCB in this emulator.");
                        TCB_MODE::TIMEOUT
                    },
                    0x02 => {
                        println!("[WARNING] CAPT mode is not implemented for TCB in this emulator.");
                        TCB_MODE::CAPT
                    },
                    0x03 => {
                        println!("[WARNING] FRQ mode is not implemented for TCB in this emulator.");
                        TCB_MODE::FRQ
                    },
                    0x04 => {
                        println!("[WARNING] PW mode is not implemented for TCB in this emulator.");
                        TCB_MODE::PW
                    },
                    0x05 => {
                        println!("[WARNING] FRQPW mode is not implemented for TCB in this emulator.");
                        TCB_MODE::FRQPW
                    },
                    0x06 => {
                        println!("[WARNING] SINGLE mode is not implemented for TCB in this emulator.");
                        TCB_MODE::SINGLE
                    },
                    0x07 => {
                        println!("[WARNING] PWM8 mode is not implemented for TCB in this emulator.");
                        TCB_MODE::PWM8
                    },
                    _ => TCB_MODE::PWM8
                };
                if value & 0x70 != 0 {
                    println!("[WARNING] ASYNC/CCMPINIT/CCMPEN features are not implemented for TCB in this emulator. These bits will be ignored.");
                }
            },
            TCB_EVCTRL => {
                println!("[WARNING] EVECTRL features are not implemented for TCB in this emulator. This register will be ignored.");
                self.regs[TCB_EVCTRL] = value;
            },
            TCB_DBGCTRL => {
                println!("[WARNING] DBGCTRL features are not implemented for TCB in this emulator. This register will be ignored.");
                self.regs[TCB_DBGCTRL] = value;
            },
            TCB_INTFLAGS => self.regs[TCB_INTFLAGS] &= !value,
            TCB_INTCTRL => self.regs[TCB_INTCTRL] = value,
            TCB_CCMPL => self.regs[TCB_TEMP] = value,
            TCB_CCMPH=> { self.regs[TCB_CCMPH] = value; self.regs[TCB_CCMPL] = self.regs[TCB_TEMP] },
            TCB_CNTL => self.regs[TCB_TEMP] = value,
            TCB_CNTH=> { self.regs[TCB_CNTH] = value; self.regs[TCB_CNTL] = self.regs[TCB_TEMP] },
            _ => {}
        }
        0
    }
}

impl InterruptSource for Tcb {
    fn interrupt(&mut self, mask: u8) -> bool {
        (self.regs[TCB_INTCTRL] & self.regs[TCB_INTFLAGS] & mask) != 0x00
    }
}

impl Clocked for Tcb {
    fn tick(&mut self, time: u64) {
        // If not enabled we do nothing
        if self.enabled {
            match self.clksel {
                TCB_CLKSEL::DIV1 => {},
                TCB_CLKSEL::DIV2 => {
                    self.tictoc = !self.tictoc;
                    if self.tictoc {return}; // Only continue every second tick   
                },
                _ => return // No other clock modes implemented
            }

            match self.cntmode {
                TCB_MODE::INT => {
                    //Increment counter
                    let mut ovf;
                    (self.regs[TCB_CNTL], ovf) = self.regs[TCB_CNTL].overflowing_add(1);
                    if ovf {
                        (self.regs[TCB_CNTH], ovf) = self.regs[TCB_CNTH].overflowing_add(1);
                    }
                    // Compare match
                    if (self.regs[TCB_CNTL] == self.regs[TCB_CCMPL]) & (self.regs[TCB_CNTH] == self.regs[TCB_CCMPH]) {
                        //println!("[{}] TCB INTFLAGS.CAPT set @{:08X}", self.name, time);
                        self.regs[TCB_INTFLAGS] |= 0x01;
                        // Reset counter
                        // TODO: Is this correct or early by a cycle?
                        self.regs[TCB_CNTL] = 0;
                        self.regs[TCB_CNTH] = 0;
                    }
                    // Overflow
                    if ovf {
                        //println!("[{}] TCB INTFLAGS.OVF set @{:08X}", self.name, time);
                        self.regs[TCB_INTFLAGS] |= 0x02;
                    }


                },
                _ => return // No other modes implemented
            }

        }
    }
}