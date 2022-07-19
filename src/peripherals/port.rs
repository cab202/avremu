use super::super::nets::{Net, NetState, PinState};
use super::super::memory::MemoryMapped;
use super::Clocked;


const PORT_DIR:     usize = 0x00;
const PORT_DIRSET:  usize = 0x01;
const PORT_DIRCLR:  usize = 0x02;
const PORT_DIRTGL:  usize = 0x03;
const PORT_OUT:     usize = 0x04;
const PORT_OUTSET:  usize = 0x05;
const PORT_OUTCLR:  usize = 0x06;
const PORT_OUTTGL:  usize = 0x07;
const PORT_IN:      usize = 0x08;
const PORT_INTFLAGS:usize = 0x09;
const PORT_PORTCTRL:usize = 0x0A;

const PORT_PIN0CTRL:usize = 0x10;
//const PORT_PIN1CTRL:usize = 0x11;
//const PORT_PIN2CTRL:usize = 0x12;
//const PORT_PIN3CTRL:usize = 0x13;
//const PORT_PIN4CTRL:usize = 0x14;
//const PORT_PIN5CTRL:usize = 0x15;
//const PORT_PIN6CTRL:usize = 0x16;
const PORT_PIN7CTRL:usize = 0x17;

enum ISC {
    INTDISABLE,
    BOTHEDGES,
    RISING,
    FALLING,
    INPUTDISABLE,
    LEVEL, 
    RESERVED
}

impl ISC {
    fn from(val: u8) -> ISC {
        match val {
            0 => ISC::INTDISABLE,
            1 => ISC::BOTHEDGES,
            2 => ISC::RISING,
            3 => ISC::FALLING,
            4 => ISC::INPUTDISABLE,
            5 => ISC::LEVEL,
            _ => ISC::RESERVED
        }
    }
}

struct PortIO {
    pin: PinState,
    dir: bool,
    out: bool, 
    pullup_en: bool, 
    invert_en: bool,
    input_dis: bool,
    isc: ISC
    //peripheral_overrides
    //interrupt sink
    //analog sink
    //pin state
}

impl PortIO {
    fn update_pinstate(&mut self) {
        if self.dir {
            // driven
            if (self.out) {
                self.pin = PinState::DriveH;
            } else {
                self.pin = PinState::DriveL;
            }
        } else {
            // not driven
            if self.pullup_en {
                self.pin = PinState::WeakPullUp;
            } else {
                self.pin = PinState::Open;
            }
        }
    }
}

struct Port {
    pio: [PortIO; 8],
    regs: [u8; 0x18]
}

impl Port {
    fn update_dir(&mut self) {
        for i in 0..8usize {
            self.pio[i].dir = self.regs[PORT_DIR] & (1 << i) != 0;
        }
    }

    fn update_out(&mut self) {
        for i in 0..8usize {
            self.pio[i].out = self.regs[PORT_OUT] & (1 << i) != 0;
        }
    }

    fn update_pinctrl(&mut self, n: usize) {
        self.pio[n].invert_en = self.regs[PORT_PIN0CTRL+n] & (0x80) != 0;
        self.pio[n].pullup_en = self.regs[PORT_PIN0CTRL+n] & (0x08) != 0;
        self.pio[n].isc = ISC::from(self.regs[PORT_PIN0CTRL+n] & 0x07);
        if let ISC::INPUTDISABLE = self.pio[n].isc {
            self.pio[n].input_dis = true;
        }
    }
}

impl MemoryMapped for Port {
    fn get_size(&self) -> usize {
        0x18    
    }

    fn read(&self, address: usize) -> (u8, usize) {
        match address {
            PORT_DIR..=PORT_DIRTGL => (self.regs[PORT_DIR], 0),
            PORT_OUT..=PORT_OUTTGL => (self.regs[PORT_OUT], 0),
            PORT_IN => (self.regs[PORT_IN], 0), //TODO: update reg value on pin status change
            PORT_INTFLAGS => (self.regs[PORT_INTFLAGS], 0),
            PORT_PORTCTRL => (self.regs[PORT_PORTCTRL] & 0x01, 0),
            PORT_PIN0CTRL..=PORT_PIN7CTRL => (self.regs[address] & 0x8F, 0),
            _ => panic!("Attenpt to access invalid register in PORT peripheral.")
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            PORT_DIR => {self.regs[PORT_DIR] = value; self.update_dir()},
            PORT_DIRSET => {self.regs[PORT_DIR] |= value; self.update_dir()},
            PORT_DIRCLR => {self.regs[PORT_DIR] &= !value; self.update_dir()},
            PORT_DIRTGL => {self.regs[PORT_DIR] ^= value; self.update_dir()},
            PORT_OUT => {self.regs[PORT_OUT] = value; self.update_out()},
            PORT_OUTSET => {self.regs[PORT_OUT] |= value; self.update_out()},
            PORT_OUTCLR => {self.regs[PORT_OUT] &= !value; self.update_out()},
            PORT_OUTTGL | PORT_IN => {self.regs[PORT_OUT] ^= value; self.update_out()},
            PORT_INTFLAGS => {self.regs[PORT_INTFLAGS] &= !value},
            PORT_PORTCTRL => {self.regs[PORT_PORTCTRL] = value & 0x01},
            PORT_PIN0CTRL..=PORT_PIN7CTRL => {self.regs[address] = value & 0x8F; self.update_pinctrl(address-PORT_PIN0CTRL)},  
            _ => panic!("Attenpt to access invalid register in PORT peripheral.")
        }
        0
    }
}

impl Clocked for Port {
    fn tick() {

    }
}