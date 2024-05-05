use crate::memory::MemoryMapped;

const PORTMUX_EVSYSROUTEA: usize = 0x00;
const PORTMUX_TCBROUTEA: usize = 0x05;

#[allow(dead_code)]
pub struct Portmux {
    name: String,
    regs: [u8; 6],
}

impl Portmux {
    pub fn new(name: String) -> Self {
        Portmux { name, regs: [0; 6] }
    }
}

impl MemoryMapped for Portmux {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            PORTMUX_EVSYSROUTEA..=PORTMUX_TCBROUTEA => {
                println!("[WARNING] PORTMUX is not implemented in this emulator. Reads of PORTMUX registers will return last written value.");
                (self.regs[address], 0)
            }
            _ => (0, 0),
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        if let PORTMUX_EVSYSROUTEA..=PORTMUX_TCBROUTEA = address {
            self.regs[address] = value;
            println!("[WARNING] PORTMUX is not implemented in this emulator. Writes to PORTMUX registers will have no effect.");
        }
        0
    }
}
