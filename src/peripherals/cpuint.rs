use std::cell::RefCell;
use std::rc::Rc;

use crate::cores::InterruptHandler;
use crate::memory::MemoryMapped;
use crate::peripherals::InterruptSource;

const _CPUINT_CTRLA: usize = 0x00;
const CPUINT_STATUS: usize = 0x01;
const _CPUINT_LVL0PRI: usize = 0x02;
const _CPUINT_LVL1VEC: usize = 0x03;

#[allow(dead_code)]
pub struct Cpuint {
    regs: [u8; 4],
    ccp: bool,
    sources: Vec<(usize, Rc<RefCell<dyn InterruptSource>>, u8)>,
    vectors: Vec<u16>,
}

impl Cpuint {
    pub fn new() -> Self {
        let mut table = Vec::new();
        for i in 0u16..30 {
            table.push(i << 1);
        }
        Cpuint {
            regs: [0; 4],
            ccp: false,
            sources: Vec::new(),
            vectors: table,
        }
    }

    pub fn add_source(
        &mut self,
        vector_index: usize,
        peripheral: Rc<RefCell<dyn InterruptSource>>,
        flag_mask: u8,
    ) {
        self.sources.push((vector_index, peripheral, flag_mask));
    }

    #[allow(dead_code)]
    pub fn ccp_enable(&mut self) {
        self.ccp = true;
    }

    #[allow(dead_code)]
    pub fn ccp_disable(&mut self) {
        self.ccp = false;
    }
}

impl MemoryMapped for Cpuint {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        (self.regs[address], 0)
    }

    fn write(&mut self, _address: usize, _value: u8) -> usize {
        println!("[WARNING] Configuration of CPUINT is not currently supported. Writes to these registers are ignored.");
        0
    }
}

impl InterruptHandler for Cpuint {
    fn service_pending(&mut self) -> Option<u16> {
        // If we are currently servicing an interrupt, can't interrupt again
        //TODO: Handle NMI and priorities correctly
        if self.regs[CPUINT_STATUS] & 0x01 == 0 {
            for i in 0..self.sources.len() {
                if self.sources[i].1.borrow_mut().interrupt(self.sources[i].2) {
                    // Set LVL0EX flag
                    self.regs[CPUINT_STATUS] |= 0x01;
                    //TODO: Handle NMI and priorities correctly
                    return Option::Some(self.vectors[self.sources[i].0]);
                }
            }
        }
        Option::None
    }

    fn reti(&mut self) {
        // Clear LVL0EX flag
        // TODO: Handle interrupt priorities correctly
        self.regs[CPUINT_STATUS] &= !0x01;
    }
}
