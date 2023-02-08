use std::rc::Rc;
use std::cell::RefCell;

use crate::memory::MemoryMapped;

use super::{Ccp, Clocked};

const CPU_CCP:  usize = 0x04;
const _CPU_SP:   usize = 0x0D;
const _CP_SREG:  usize = 0x0F;

pub struct Cpu {
    regs: [u8; 0x10],
    ccp_ioreg: Vec<Rc<RefCell<dyn Ccp>>>,
    ccp_ioreg_count: u8
}

impl Cpu {
    pub fn new(ccp_ioreg: Vec<Rc<RefCell<dyn Ccp>>>) -> Self {
        Cpu {
            regs: [0; 0x10],
            ccp_ioreg,
            ccp_ioreg_count: 0
        }
    }
}

impl MemoryMapped for Cpu {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            CPU_CCP => if self.ccp_ioreg_count > 0 {(1, 0)} else {(0, 0)}
            _ => (self.regs[address], 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            CPU_CCP => if value == 0xD8 {
                self.ccp_ioreg_count = 4;
                for ccp in &self.ccp_ioreg {
                    ccp.borrow_mut().ccp(true);
                }
            },
            _ => {}
        }
        0
    }
}

impl Clocked for Cpu {
    fn tick(&mut self, _time: u64) {
        if self.ccp_ioreg_count > 0 {
            self.ccp_ioreg_count -= 1;

            if self.ccp_ioreg_count == 0 {
                for ccp in &self.ccp_ioreg {
                    ccp.borrow_mut().ccp(false);
                }
            }
        }
        
    }
} 