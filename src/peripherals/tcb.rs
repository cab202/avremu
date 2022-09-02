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

pub struct Tcb {
    name: String, 
    regs: [u8; 0x0E]
}

impl Tcb {
    pub fn new(name: String) -> Self {
        Tcb {
            name,
            regs: [0; 0x0E]
        }
    }
}

impl MemoryMapped for Tcb {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            TCB_INTFLAGS..=TCB_TEMP => (self.regs[address], 0),
            TCB_CCMPL => { self.regs[TCB_TEMP] = self.regs[TCB_CCMPH]; (self.regs[TCB_CCMPL], 0) },
            TCB_CCMPH=>  (self.regs[TCB_TEMP], 0),
            TCB_CNTL => { self.regs[TCB_TEMP] = self.regs[TCB_CNTH]; (self.regs[TCB_CNTL], 0) },
            TCB_CNTH=> (self.regs[TCB_TEMP], 0),
            _ => (0, 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
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
    fn interrupt(&self, index: usize) -> bool {
        match index {
            0 | 1 => {
                (self.regs[TCB_INTCTRL] & self.regs[TCB_INTFLAGS] & 0x03) != 0x00
            },
            _ => false
        }
    }
}

impl Clocked for Tcb {
    fn tick(&mut self, time: usize) {
        (self.regs[TCB_CNTL], _) = self.regs[TCB_CNTL].overflowing_add(1);
        if self.regs[TCB_CNTL] == 0 {
            self.regs[TCB_INTFLAGS] |= 0x02;
        }
    }
}