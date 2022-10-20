use std::cell::RefCell;
use std::rc::Rc;

use crate::memory::MemoryMapped;
use crate::nets::NetState;
use crate::peripherals::InterruptSource;
use crate::peripherals::Clocked;

use super::port::Port;

const ADC_CTRLA:    usize = 0x00;
const ADC_CTRLB:    usize = 0x01;
const ADC_CTRLC:    usize = 0x02;
const ADC_CTRLD:    usize = 0x03;
const ADC_INTCTRL:  usize = 0x04;
const ADC_INTFLAGS: usize = 0x05;
const ADC_STATUS:   usize = 0x06;
const ADC_DBGCTRL:  usize = 0x07;
const ADC_CTRLE:    usize = 0x08;
const ADC_CTRLF:    usize = 0x09;
const ADC_COMMAND:  usize = 0x0A;
const ADC_PGACTRL:  usize = 0x0B;
const ADC_MUXPOS:   usize = 0x0C;
const ADC_MUXNEG:   usize = 0x0D;
const ADC_RESULT0:  usize = 0x10;
const ADC_RESULT1:  usize = 0x11;
const ADC_RESULT2:  usize = 0x12;
const ADC_RESULT3:  usize = 0x13;
const ADC_SAMPLEL:  usize = 0x14;
const ADC_SAMPLEH:  usize = 0x15;
const ADC_TEMP0:    usize = 0x18;
const ADC_TEMP1:    usize = 0x19;
const ADC_TEMP2:    usize = 0x1A;
const ADC_WINLTL:   usize = 0x1C;
const ADC_WINLTH:   usize = 0x1D;
const ADC_WINHTL:   usize = 0x1E;
const ADC_WINHTH:   usize = 0x1F;

#[derive(PartialEq)]
enum ADC_MODE {
    SINGLE_8BIT,
    SINGLE_12BIT,
    SERIES, 
    SERIES_SCALING, 
    BURST,
    BURST_SCALING,
    RESERVED
}

enum ADC_REFSEL {
    VDD,
    VREFA,
    V1024, 
    V2048,
    V2500,
    V4096,
    RESERVED
}

pub struct Adc {
    name: String, 
    regs: [u8; 0x20],
    enabled: bool,
    presc: u8,
    mode: ADC_MODE,
    vref: ADC_REFSEL,
    clk_divider: u8,
    ports: [Rc<RefCell<Port>>; 3],
    ain: [(usize, u8); 15],
    muxpos: usize,
    busy: bool,
    delay: usize,
    sample: u16
}

impl Adc {
    pub fn new(name: String, ports: [Rc<RefCell<Port>>; 3], ain: [(usize, u8); 15]) -> Self {
        Adc {
            name,
            regs: [0; 0x20],
            enabled: false,
            presc: 2,
            mode: ADC_MODE::SINGLE_8BIT,
            vref: ADC_REFSEL::VDD,
            clk_divider: 0,
            ports,
            ain,
            muxpos: 0,
            busy: false,
            delay: 0,
            sample: 0
        }
    }

    fn sample(&mut self) {
        if !self.busy {
            self.busy = true;
            self.delay = self.regs[ADC_CTRLE] as usize + if self.mode.eq(&ADC_MODE::SINGLE_8BIT) {9} else {13};
            let ainp = self.muxpos;
            self.sample = if self.muxpos == 0 {
                0
            } else {
                let (portidx, pinidx) = self.ain[ainp-1];
                let vref = match self.vref {
                    ADC_REFSEL::VDD => 3.3,
                    ADC_REFSEL::V1024 => 1.024,
                    ADC_REFSEL::V2048 => 2.048,
                    ADC_REFSEL::V2500 => 2.5,
                    ADC_REFSEL::V4096 => 2.8,
                    _ => 0.001
                };
                match self.ports[portidx].borrow_mut().get_netstate(pinidx) {
                    NetState::High => 0x0FFF,
                    NetState::Low => 0x0000,
                    NetState::Undefined => 0x0000,
                    NetState::Analog(voltage) => {
                        let mut conversion = 4096.0*voltage/vref; 
                        if conversion > 4095.0 {conversion = 4095.0};
                        conversion as u16                   
                    }
                }
            }
        }
    } 
}

impl MemoryMapped for Adc {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            ADC_CTRLA..=ADC_INTFLAGS | ADC_DBGCTRL..=ADC_MUXNEG | ADC_TEMP0..=ADC_TEMP2 => (self.regs[address], 0),
            ADC_STATUS => if self.busy {(1, 0)} else {(0, 0)},
            ADC_RESULT0 => { 
                self.regs[ADC_TEMP0] = self.regs[ADC_RESULT1];
                self.regs[ADC_TEMP1] = self.regs[ADC_RESULT2]; 
                self.regs[ADC_TEMP2] = self.regs[ADC_RESULT3];  
                (self.regs[ADC_RESULT0], 0) 
            },
            ADC_RESULT1=> (self.regs[ADC_TEMP0], 0),
            ADC_RESULT2=> (self.regs[ADC_TEMP1], 0),
            ADC_RESULT3=> (self.regs[ADC_TEMP2], 0),
            ADC_SAMPLEL => { self.regs[ADC_TEMP0] = self.regs[ADC_SAMPLEH]; (self.regs[ADC_SAMPLEL], 0) },
            ADC_SAMPLEH=> (self.regs[ADC_TEMP0], 0),
            ADC_WINLTL => { self.regs[ADC_TEMP0] = self.regs[ADC_WINLTH]; (self.regs[ADC_WINLTL], 0) },
            ADC_WINLTH=> (self.regs[ADC_TEMP0], 0),
            ADC_WINHTL => { self.regs[ADC_TEMP0] = self.regs[ADC_WINHTH]; (self.regs[ADC_WINHTL], 0) },
            ADC_WINHTH=> (self.regs[ADC_TEMP0], 0),
            _ => (0, 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            ADC_CTRLA => {
                self.regs[ADC_CTRLA] = value;
                self.enabled = (value & 0x01) != 0;
                if value & 0xFE != 0 {
                    println!("[WARNING] RUNSTDBY/LOWLAT features are not implemented for ADC in this emulator. These bits will be ignored.");
                }
            },
            ADC_CTRLB => {
                self.regs[ADC_CTRLB] = value;
                self.presc = match value & 0x0F {
                    0x0..=0x7 => (self.presc<<1)+1,
                    0x8..=0xB => (self.presc<<2)+3,
                    0xC..=0xF => ((self.presc-12)<<3)+39,
                    _ => 1
                }
            },
            ADC_CTRLC => {
                self.regs[ADC_CTRLC] = value;
                self.vref = match value & 0x07 {
                    0x0 => ADC_REFSEL::VDD,
                    0x2 => ADC_REFSEL::VREFA,
                    0x4 => ADC_REFSEL::V1024,
                    0x5 => ADC_REFSEL::V2048,
                    0x6 => ADC_REFSEL::V2500,
                    0x7 => ADC_REFSEL::V4096,
                    _ => ADC_REFSEL::RESERVED
                };
                if (value >> 3) < 4 {
                    //TODO: Fix hardcoding for 3.3 MHz
                    println!("[WARNING] Inappropriate TIMEBASE value specified for ADC: {}.", value >> 3);
                }
            },
            ADC_CTRLD => {
                println!("[WARNING] CTRLD features are not implemented for ADC in this emulator. This register will be ignored.");
                self.regs[ADC_CTRLD] = value;
            },
            ADC_INTCTRL => {
                if value > 1 {
                    println!("[WARNING] Only RESRDY is implemented for ADC in this emulator. No other interrupt flags will be set.");
                }
                self.regs[ADC_INTCTRL] = value;
            },
            ADC_INTFLAGS => self.regs[ADC_INTFLAGS] &= !value,
            ADC_DBGCTRL => {
                println!("[WARNING] DBGCTRL features are not implemented for ADC in this emulator. This register will be ignored.");
                self.regs[ADC_DBGCTRL] = value;
            },
            ADC_CTRLE => {
                self.regs[ADC_CTRLE] = value;
            },
            ADC_CTRLF => {
                self.regs[ADC_CTRLF] = value;
                if value & 0x0F != 0 {
                    println!("[WARNING] SAMPNUM features are not implemented for ADC in this emulator. These bits will be ignored.");
                }
            },
            ADC_COMMAND => {
                self.regs[ADC_COMMAND] = value & 0x7F;
                if value & 0x80 != 0 {
                    println!("[WARNING] DIFF feature is not implemented for ADC in this emulator. This bit will be ignored.");
                }
                match value & 0x07 {
                    0x00 => {self.busy = false},
                    0x01 => {
                        // Reset to STOP if not enabled
                        if self.regs[ADC_CTRLA] & 0x01 == 0 {
                            self.regs[ADC_COMMAND] &= 0xF8; 
                        } else {
                            self.sample();    
                        }
                    },
                    _ => {
                        self.regs[ADC_COMMAND] &= 0xF8; 
                        println!("[WARNING] Only IMMEDIATE start trigger is implemented for ADC in this emulator. Write to START field will be ignored.");
                    }
                }
                match (value >> 4) & 0x07 {
                    0x00 => self.mode = ADC_MODE::SINGLE_8BIT,
                    0x01 => self.mode = ADC_MODE::SINGLE_12BIT,
                    _ => {
                        self.mode = ADC_MODE::RESERVED; 
                        println!("[WARNING] Accumulation modes are not implemented for ADC in this emulator. ADC will not be functional.");
                    }
                }
            },
            ADC_PGACTRL => {
                println!("[WARNING] PGACTRL features are not implemented for ADC in this emulator. This register will be ignored.");
                self.regs[ADC_PGACTRL] = value;
            },
            ADC_MUXPOS => {
                self.regs[ADC_MUXPOS] = value;
                // VIA field is common
                self.regs[ADC_MUXNEG] &= 0x3F;
                self.regs[ADC_MUXNEG] |= value & 0xC0;
                match value & 0x3F {
                    0..=16 => self.muxpos = (value & 0x3F) as usize,
                    0x30 => self.muxpos = 0,
                    _ => {
                        println!("[WARNING] Only AIN1..15 and GND MUXPOS selections are implemented for ADC. MUXPOS will revert to GND.");
                        self.muxpos = 0;
                    }
                }
            },
            ADC_MUXNEG => {
                println!("[WARNING] MUXNEG features are not implemented for ADC in this emulator. This register will be ignored.");
                self.regs[ADC_MUXNEG] = value;
                // VIA field is common
                self.regs[ADC_MUXPOS] &= 0x3F;
                self.regs[ADC_MUXPOS] |= value & 0xC0;
            },
            ADC_RESULT0 => self.regs[ADC_TEMP0] = value,
            ADC_RESULT1 => self.regs[ADC_TEMP1] = value,
            ADC_RESULT2 => self.regs[ADC_TEMP2] = value,
            ADC_RESULT3=> { 
                self.regs[ADC_RESULT3] = value; 
                self.regs[ADC_RESULT2] = self.regs[ADC_TEMP2];
                self.regs[ADC_RESULT1] = self.regs[ADC_TEMP1];
                self.regs[ADC_RESULT0] = self.regs[ADC_TEMP0]; 
            },
            ADC_SAMPLEL => self.regs[ADC_TEMP0] = value,
            ADC_SAMPLEH=> { self.regs[ADC_SAMPLEH] = value; self.regs[ADC_SAMPLEL] = self.regs[ADC_TEMP0] },
            ADC_TEMP0 => self.regs[ADC_TEMP0] = value,
            ADC_TEMP1 => self.regs[ADC_TEMP1] = value,
            ADC_TEMP2 => self.regs[ADC_TEMP2] = value,
            ADC_WINLTL => self.regs[ADC_TEMP0] = value,
            ADC_WINLTH=> { self.regs[ADC_WINLTH] = value; self.regs[ADC_WINLTL] = self.regs[ADC_TEMP0] },
            ADC_WINHTL => self.regs[ADC_TEMP0] = value,
            ADC_WINHTH=> { self.regs[ADC_WINHTH] = value; self.regs[ADC_WINHTL] = self.regs[ADC_TEMP0] },
            _ => {}
        }
        0
    }
}

impl InterruptSource for Adc {
    fn interrupt(&mut self, mask: u8) -> bool {
        (self.regs[ADC_INTCTRL] & self.regs[ADC_INTFLAGS] & mask) != 0x00
    }
}

impl Clocked for Adc {
    fn tick(&mut self, time: u64) {
        // If not enabled we do nothing
        if self.enabled {
            if self.clk_divider > 0 {
                self.clk_divider -= 1;
                // Only clocked when divide == 0
                return;
            }
            self.clk_divider = self.presc; 
            if self.busy {
                // Conversion in process
                if self.delay == 0 {
                    // Conversion complete
                    self.busy = false;
                    //println!("[ADC0] Conversion complete: 0x{:06X}", self.sample);
                    match self.mode {
                        ADC_MODE::SINGLE_8BIT => {
                            self.regs[ADC_RESULT0] = (self.sample >> 4) as u8;
                            self.regs[ADC_RESULT1] = 0;
                            self.regs[ADC_RESULT2] = 0;
                            self.regs[ADC_RESULT3] = 0;
                        },
                        ADC_MODE::SINGLE_12BIT => {
                            if (self.regs[ADC_CTRLF] & 0x10) != 0 {
                                // Left adjust
                                self.regs[ADC_RESULT0] = ((self.sample << 4) & 0xFF) as u8;
                                self.regs[ADC_RESULT1] = (self.sample >> 4) as u8;
                                self.regs[ADC_RESULT2] = 0;
                                self.regs[ADC_RESULT3] = 0;
                            } else {
                                self.regs[ADC_RESULT0] = (self.sample & 0xFF) as u8;
                                self.regs[ADC_RESULT1] = (self.sample >> 8) as u8;
                                self.regs[ADC_RESULT2] = 0;
                                self.regs[ADC_RESULT3] = 0;
                            }
                        },
                        _ => {} // No other modes implemented
                    }
                    self.regs[ADC_INTFLAGS] |= 0x01;
                    if (self.regs[ADC_CTRLF] & 0x20) != 0 {
                        // Free running
                        self.sample();
                    } else {
                        self.regs[ADC_COMMAND] &= 0xF8; //STOP
                    }
                } else {
                    self.delay -= 1;
                }
            }
            
        }
    }
}