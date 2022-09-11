use std::cell::RefCell;
use std::rc::Rc;

use crate::memory::MemoryMapped;
use crate::peripherals::InterruptSource;
use crate::peripherals::Clocked;

use super::port::Port;

const TCA_CTRLA:    usize = 0x00;
const TCA_CTRLB:    usize = 0x01;
const TCA_CTRLC:    usize = 0x02;
const TCA_CTRLD:    usize = 0x03;
const TCA_CTRLECLR: usize = 0x04;
const TCA_CTRLESET: usize = 0x05;
const TCA_CTRLFCLR: usize = 0x06;
const TCA_CTRLFSET: usize = 0x07;
const TCA_EVCTRL:   usize = 0x09;
const TCA_INTCTRL:  usize = 0x0A;
const TCA_INTFLAGS: usize = 0x0B;
const TCA_DBGCTRL:  usize = 0x0E;
const TCA_TEMP:     usize = 0x0F;
const TCA_CNTL:     usize = 0x20;
const TCA_CNTH:     usize = 0x21;
const TCA_PERL:     usize = 0x26;
const TCA_PERH:     usize = 0x27;
const TCA_CMP0L:    usize = 0x28;
const TCA_CMP0H:    usize = 0x29;
const TCA_CMP1L:    usize = 0x2A;
const TCA_CMP1H:    usize = 0x2B;
const TCA_CMP2L:    usize = 0x2C;
const TCA_CMP2H:    usize = 0x2D;
const TCA_PERBUFL:  usize = 0x36;
const TCA_PERBUFH:  usize = 0x37;
const TCA_CMP0BUFL: usize = 0x38;
const TCA_CMP0BUFH: usize = 0x39;
const TCA_CMP1BUFL: usize = 0x3A;
const TCA_CMP1BUFH: usize = 0x3B;
const TCA_CMP2BUFL: usize = 0x3C;
const TCA_CMP2BUFH: usize = 0x3D;

enum TCA_MODE {
    NORMAL,
    FRQ, 
    SINGLESLOPE,
    DSTOP,
    DSBOTH,
    DSBOTTOM
}

enum TCA_CLKSEL {
    DIV1,
    DIV2,
    DIV4,
    DIV8,
    DIV16,
    DIV64,
    DIV256,
    DIV1024
}

pub struct Tca {
    name: String, 
    regs: [u8; 0x3E],
    enabled: bool,
    clksel: TCA_CLKSEL,
    cntmode: TCA_MODE,
    clk_divider: u16,
    port: Rc<RefCell<Port>>,
    pins: [u8; 3],
    pins_alt: [u8; 3],
    pub mux_alt: [bool; 3],
}

impl Tca {
    pub fn new(name: String, port: Rc<RefCell<Port>>, pins: [u8; 3], pins_alt: [u8; 3]) -> Self {
        Tca {
            name,
            regs: [0; 0x3E],
            enabled: false,
            clksel: TCA_CLKSEL::DIV1,
            cntmode: TCA_MODE::NORMAL,
            clk_divider: 0,
            port,
            pins,
            pins_alt,
            mux_alt: [false; 3]
        }
    }
}

impl MemoryMapped for Tca {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            TCA_CTRLA..=TCA_CTRLD => (self.regs[address], 0),
            TCA_CTRLECLR..=TCA_CTRLESET => (self.regs[TCA_CTRLECLR] & 0x03, 0),
            TCA_CTRLFCLR..=TCA_CTRLFSET => (self.regs[TCA_CTRLFCLR], 0),
            TCA_EVCTRL..=TCA_TEMP => (self.regs[address], 0),
            TCA_CNTL => { self.regs[TCA_TEMP] = self.regs[TCA_CNTH]; (self.regs[TCA_CNTL], 0) },
            TCA_CNTH=> (self.regs[TCA_TEMP], 0),
            TCA_PERL => { self.regs[TCA_TEMP] = self.regs[TCA_PERH]; (self.regs[TCA_PERL], 0) },
            TCA_PERH=> (self.regs[TCA_TEMP], 0),
            TCA_CMP0L => { self.regs[TCA_TEMP] = self.regs[TCA_CMP0H]; (self.regs[TCA_CMP0L], 0) },
            TCA_CMP0H=> (self.regs[TCA_TEMP], 0),
            TCA_CMP1L => { self.regs[TCA_TEMP] = self.regs[TCA_CMP1H]; (self.regs[TCA_CMP1L], 0) },
            TCA_CMP1H=> (self.regs[TCA_TEMP], 0),
            TCA_CMP2L => { self.regs[TCA_TEMP] = self.regs[TCA_CMP2H]; (self.regs[TCA_CMP2L], 0) },
            TCA_CMP2H=> (self.regs[TCA_TEMP], 0),
            TCA_PERBUFL => { self.regs[TCA_TEMP] = self.regs[TCA_PERBUFH]; (self.regs[TCA_PERBUFL], 0) },
            TCA_PERBUFH=> (self.regs[TCA_TEMP], 0),
            TCA_CMP0BUFL => { self.regs[TCA_TEMP] = self.regs[TCA_CMP0BUFH]; (self.regs[TCA_CMP0BUFL], 0) },
            TCA_CMP0BUFH=> (self.regs[TCA_TEMP], 0),
            TCA_CMP1BUFL => { self.regs[TCA_TEMP] = self.regs[TCA_CMP1BUFH]; (self.regs[TCA_CMP1BUFL], 0) },
            TCA_CMP1BUFH=> (self.regs[TCA_TEMP], 0),
            TCA_CMP2BUFL => { self.regs[TCA_TEMP] = self.regs[TCA_CMP2BUFH]; (self.regs[TCA_CMP2BUFL], 0) },
            TCA_CMP2BUFH=> (self.regs[TCA_TEMP], 0),
            _ => (0, 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            TCA_CTRLA => {
                self.regs[TCA_CTRLA] = value;
                self.enabled = (value & 0x01) != 0;
                self.clksel = match (value >> 1) & 0x07 {
                    0x00 => TCA_CLKSEL::DIV1,
                    0x01 => TCA_CLKSEL::DIV2,
                    0x02 => TCA_CLKSEL::DIV4,
                    0x03 => TCA_CLKSEL::DIV8,
                    0x04 => TCA_CLKSEL::DIV16,
                    0x05 => TCA_CLKSEL::DIV64,
                    0x06 => TCA_CLKSEL::DIV256,
                    0x07 => TCA_CLKSEL::DIV1024,
                    _ => TCA_CLKSEL::DIV1
                };
                if value & 0x80 != 0 {
                    println!("[WARNING] RUNSTDBY feature is not implemented for TCA in this emulator. This bit will be ignored.");
                }
            },
            TCA_CTRLB => {
                self.regs[TCA_CTRLB] = value;
                for i in 0..3 {
                    if ((value >> 4) & (1 << i)) == 0 {
                        //WO disabled
                        if self.mux_alt[i] {
                            self.port.borrow_mut().po_out_clear(self.pins_alt[i]);
                        } else {
                            self.port.borrow_mut().po_out_clear(self.pins[i]);
                        }
                    } else {
                        //WO enabled
                        let wo = (self.regs[TCA_CTRLC] & (1<<i)) != 0;
                        if self.mux_alt[i] {
                            self.port.borrow_mut().po_out(self.pins_alt[i], wo);
                        } else {
                            self.port.borrow_mut().po_out(self.pins[i], wo);
                        }
                    } 
                }
                self.cntmode = match value & 0x07 {
                    0x00 => {
                        println!("[WARNING] NORMAL mode is not implemented for TCA in this emulator.");
                        TCA_MODE::NORMAL
                    },
                    0x01 => {
                        println!("[WARNING] FRQ mode is not implemented for TCA in this emulator.");
                        TCA_MODE::FRQ
                    },
                    0x03 => TCA_MODE::SINGLESLOPE,
                    0x05 => {
                        println!("[WARNING] DSTOP mode is not implemented for TCA in this emulator.");
                        TCA_MODE::DSTOP
                    },
                    0x06 => {
                        println!("[WARNING] DSBOTH mode is not implemented for TCA in this emulator.");
                        TCA_MODE::DSBOTH
                    },
                    0x07 => {
                        println!("[WARNING] DSBOTTOM mode is not implemented for TCA in this emulator.");
                        TCA_MODE::DSBOTTOM
                    },
                    _ => {
                        println!("[WARNING] Invalid mode specified for TCA. TCA will default to NORMAL mode.");
                        TCA_MODE::NORMAL
                    },
                    
                };
                if value & 0x08 != 0 {
                    println!("[WARNING] ALUPD features are not implemented for TCA in this emulator. These bits will be ignored.");
                }
            },
            TCA_CTRLC => {
                self.regs[TCA_CTRLC] = value;
            },
            TCA_CTRLD => {
                println!("[WARNING] CTRLD features are not implemented for TCA in this emulator. This register will be ignored.");
                self.regs[TCA_CTRLD] = value;
            },
            TCA_CTRLESET => {
                println!("[WARNING] CTRLE features are not implemented for TCA in this emulator. This register will be ignored.");
                self.regs[TCA_CTRLECLR] |= value;
            },
            TCA_CTRLECLR => {
                println!("[WARNING] CTRLE features are not implemented for TCA in this emulator. This register will be ignored.");
                self.regs[TCA_CTRLECLR] &= !value;
            },
            TCA_CTRLFSET => {
                self.regs[TCA_CTRLFCLR] |= value;
            },
            TCA_CTRLFCLR => {
                self.regs[TCA_CTRLFCLR] &= !value;
            },
            TCA_EVCTRL => {
                println!("[WARNING] EVECTRL features are not implemented for TCA in this emulator. This register will be ignored.");
                self.regs[TCA_EVCTRL] = value;
            },
            TCA_INTFLAGS => self.regs[TCA_INTFLAGS] &= !value,
            TCA_INTCTRL => self.regs[TCA_INTCTRL] = value,
            TCA_DBGCTRL => {
                println!("[WARNING] DBGCTRL features are not implemented for TCA in this emulator. This register will be ignored.");
                self.regs[TCA_DBGCTRL] = value;
            },
            TCA_CNTL => self.regs[TCA_TEMP] = value,
            TCA_CNTH=> { self.regs[TCA_CNTH] = value; self.regs[TCA_CNTL] = self.regs[TCA_TEMP] },
            TCA_PERL => self.regs[TCA_TEMP] = value,
            TCA_PERH=> { self.regs[TCA_PERH] = value; self.regs[TCA_PERL] = self.regs[TCA_TEMP] },
            TCA_CMP0L => self.regs[TCA_TEMP] = value,
            TCA_CMP0H=> { self.regs[TCA_CMP0H] = value; self.regs[TCA_CMP0L] = self.regs[TCA_TEMP] },
            TCA_CMP1L => self.regs[TCA_TEMP] = value,
            TCA_CMP1H=> { self.regs[TCA_CMP1H] = value; self.regs[TCA_CMP1L] = self.regs[TCA_TEMP] },
            TCA_CMP2L => self.regs[TCA_TEMP] = value,
            TCA_CMP2H=> { self.regs[TCA_CMP2H] = value; self.regs[TCA_CMP2L] = self.regs[TCA_TEMP] },
            TCA_PERBUFL => self.regs[TCA_TEMP] = value,
            TCA_PERBUFH=> { self.regs[TCA_PERBUFH] = value; self.regs[TCA_PERBUFL] = self.regs[TCA_TEMP]; self.regs[TCA_CTRLFCLR] |= 0x01 },
            TCA_CMP0BUFL => self.regs[TCA_TEMP] = value,
            TCA_CMP0BUFH=> { self.regs[TCA_CMP0BUFH] = value; self.regs[TCA_CMP0BUFL] = self.regs[TCA_TEMP]; self.regs[TCA_CTRLFCLR] |= 0x02 },
            TCA_CMP1BUFL => self.regs[TCA_TEMP] = value,
            TCA_CMP1BUFH=> { self.regs[TCA_CMP1BUFH] = value; self.regs[TCA_CMP1BUFL] = self.regs[TCA_TEMP]; self.regs[TCA_CTRLFCLR] |= 0x04 },
            TCA_CMP2BUFL => self.regs[TCA_TEMP] = value,
            TCA_CMP2BUFH=> { self.regs[TCA_CMP2BUFH] = value; self.regs[TCA_CMP2BUFL] = self.regs[TCA_TEMP]; self.regs[TCA_CTRLFCLR] |= 0x08 },
            _ => {}
        }
        0
    }
}

impl InterruptSource for Tca {
    fn interrupt(&self, mask: u8) -> bool {
        (self.regs[TCA_INTCTRL] & self.regs[TCA_INTFLAGS] & mask) != 0x00
    }
}

impl Clocked for Tca {
    fn tick(&mut self, time: usize) {
        // If not enabled we do nothing
        if self.enabled {
            if self.clk_divider > 0 {
                self.clk_divider -= 1;
                // Only clocked when divide == 0
                return;
            } 
            match self.clksel {
                TCA_CLKSEL::DIV1 => {},
                TCA_CLKSEL::DIV2 => {self.clk_divider = 1},
                TCA_CLKSEL::DIV4 => {self.clk_divider = 3},
                TCA_CLKSEL::DIV8 => {self.clk_divider = 7},
                TCA_CLKSEL::DIV16 => {self.clk_divider = 15},
                TCA_CLKSEL::DIV64 => {self.clk_divider = 63},
                TCA_CLKSEL::DIV256 => {self.clk_divider = 255},
                TCA_CLKSEL::DIV1024 => {self.clk_divider = 1023},
            }
            match self.cntmode {
                TCA_MODE::SINGLESLOPE => {
                    //Increment counter
                    if (self.regs[TCA_CNTL] == self.regs[TCA_PERL]) & (self.regs[TCA_CNTH] == self.regs[TCA_PERH]) {
                        // Reset
                        self.regs[TCA_CNTL] = 0;
                        self.regs[TCA_CNTH] = 0;
                    } else {
                        let ovf;
                        (self.regs[TCA_CNTL], ovf) = self.regs[TCA_CNTL].overflowing_add(1);
                        if ovf {
                            self.regs[TCA_CNTH] += 1;
                        }
                    }
                    
                    // BOTTOM
                    if (self.regs[TCA_CNTL] == 0) & (self.regs[TCA_CNTH] == 0) {
                        
                        if (self.regs[TCA_CTRLFCLR] & 0x01) != 0 {
                            self.regs[TCA_PERL] = self.regs[TCA_PERBUFL];
                            self.regs[TCA_PERH] = self.regs[TCA_PERBUFH];
                        }

                        for i in 0..3 {
                            if (self.regs[TCA_CTRLFCLR] & (0x02<<i)) != 0 {
                                self.regs[TCA_CMP0L+(i<<1)] = self.regs[TCA_CMP0BUFL+(i<<1)];
                                self.regs[TCA_CMP0H+(i<<1)] = self.regs[TCA_CMP0BUFH+(i<<1)];
                            }

                            self.regs[TCA_CTRLC] |= (1<<i); // Set WO
                        }

                        self.regs[TCA_CTRLFCLR] &= 0xF0; // update event, clear BV bits
                    } 
                    
                    //TOP
                    if (self.regs[TCA_CNTL] == self.regs[TCA_PERL]) & (self.regs[TCA_CNTH] == self.regs[TCA_PERH]) {
                        //println!("[{}] TCA INTFLAGS.OVF set @{:08X}", self.name, time);
                        self.regs[TCA_INTFLAGS] |= 0x01;
                    }

                    // Compare match
                    for i in 0..3 {
                        if (self.regs[TCA_CNTL] == self.regs[TCA_CMP0L+(i<<1)]) & (self.regs[TCA_CNTH] == self.regs[TCA_CMP0H+(i<<1)]) {
                            self.regs[TCA_CTRLC] &= !(1<<i); // Clear WO
                            //println!("[{}] TCA INTFLAGS.CMP{} set @{:08X}", self.name, i, time);
                            self.regs[TCA_INTFLAGS] |= 0x10<<i;
                        }
                    }

                },
                _ => return // No other modes implemented
            }

        }

        // Port overrides
        // We update pins regardless of whether TCA is enabled
        for i in 0..3 {
            if (self.regs[TCA_CTRLB] & (0x10<<i)) != 0 {
                //WO channel enabled
                let wo = (self.regs[TCA_CTRLC] & (1<<i)) != 0;
                if self.mux_alt[i] {
                    self.port.borrow_mut().po_out(self.pins_alt[i], wo);
                } else {
                    self.port.borrow_mut().po_out(self.pins[i], wo);
                }
            }
        }
    }
}