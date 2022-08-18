use std::rc::Rc;
use std::cell::RefCell;

use bitvec::view::BitView;

use crate::hardware::Hardware;
use crate::nets::{Net, NetState, PinState};
use crate::memory::MemoryMapped;

use bitvec::prelude::*;


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
    pin: Rc<RefCell<PinState>>,
    net: Rc<RefCell<Net>>,
    dir: bool,
    out: bool, 
    pullup_en: bool, 
    invert_en: bool,
    input_dis: bool,
    isc: ISC,
    po_out: bool,
    po_out_val: bool,
    po_dir: bool,
    po_dir_val: bool
    //interrupt sink
    //analog sink
    //pin state
}

impl PortIO {
    fn new() -> Self {
        PortIO { 
            pin: Rc::new(RefCell::new(PinState::Open)), 
            net: Rc::new(RefCell::new(Net::new("".to_string()))), 
            dir: false, 
            out: false, 
            pullup_en: false, 
            invert_en: false, 
            input_dis: false, 
            isc: ISC::INTDISABLE,
            po_out: false,
            po_out_val: false,
            po_dir: false,
            po_dir_val: false 
        }
    }

    fn update_pinstate(&mut self) {
        if self.dir | (self.po_dir & self.po_dir_val) {
            // driven
            if self.po_out {
                if self.po_out_val {
                    *self.pin.borrow_mut() = PinState::DriveH;
                } else {
                    *self.pin.borrow_mut() = PinState::DriveL;
                }
            } else if self.out {
                *self.pin.borrow_mut() = PinState::DriveH;
            } else {
                *self.pin.borrow_mut() = PinState::DriveL;
            }
        } else {
            // not driven
            if self.pullup_en {
                *self.pin.borrow_mut() = PinState::WeakPullUp;
            } else {
                *self.pin.borrow_mut() = PinState::Open;
            }
        }
    }

    fn connect(&mut self, net: Rc<RefCell<Net>>) {
        self.net = net;
        self.net.borrow_mut().connect(Rc::downgrade(&self.pin));
    }
}

pub struct Port {
    name: String,
    pio: [PortIO; 8],
    regs: [u8; 0x18]
}

impl Port {
    pub fn new(name: String) -> Self {
        Port {
            name,
            pio: [
                PortIO::new(),
                PortIO::new(),
                PortIO::new(),
                PortIO::new(),
                PortIO::new(),
                PortIO::new(),
                PortIO::new(),
                PortIO::new()
            ],
            regs: [0u8; 0x18]
        }
    }

    pub fn connect(&mut self, pin_index: u8, net: Rc<RefCell<Net>>) {
        self.pio[usize::from(pin_index)].connect(net);
    }

    fn update_dir(&mut self) {
        for i in 0..8usize {
            self.pio[i].dir = self.regs[PORT_DIR] & (1 << i) != 0;
            self.pio[i].update_pinstate();
        }
    }

    fn update_out(&mut self) {
        for i in 0..8usize {
            self.pio[i].out = self.regs[PORT_OUT] & (1 << i) != 0;
            self.pio[i].update_pinstate();
        }
    }

    fn update_pinctrl(&mut self, n: usize) {
        self.pio[n].invert_en = self.regs[PORT_PIN0CTRL+n] & (0x80) != 0;
        self.pio[n].pullup_en = self.regs[PORT_PIN0CTRL+n] & (0x08) != 0;
        self.pio[n].isc = ISC::from(self.regs[PORT_PIN0CTRL+n] & 0x07);
        if let ISC::INPUTDISABLE = self.pio[n].isc {
            self.pio[n].input_dis = true;
        }
        self.pio[n].update_pinstate();
    }

    pub fn po_out(&mut self, pin_index: u8, state: bool) {
        self.pio[usize::from(pin_index)].po_out_val = state;
        self.pio[usize::from(pin_index)].po_out = true; 
        self.pio[usize::from(pin_index)].update_pinstate();  
    }

    pub fn po_out_clear(&mut self, pin_index: u8) {
        self.pio[usize::from(pin_index)].po_out = false;
        self.pio[usize::from(pin_index)].update_pinstate();   
    }

    pub fn po_dir(&mut self, pin_index: u8, state: bool) {
        self.pio[usize::from(pin_index)].po_dir_val = state;
        self.pio[usize::from(pin_index)].po_dir = true; 
        self.pio[usize::from(pin_index)].update_pinstate();  
    }

    pub fn po_dir_clear(&mut self, pin_index: u8) {
        self.pio[usize::from(pin_index)].po_dir = false;
        self.pio[usize::from(pin_index)].update_pinstate();   
    }

    pub fn get_pinstate(&self, pin_index: u8) -> bool {
        self.regs[PORT_IN].view_bits::<Lsb0>()[usize::from(pin_index)]
    }
}

impl MemoryMapped for Port {
    fn get_size(&self) -> usize {
        0x18    
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            PORT_DIR..=PORT_DIRTGL => (self.regs[PORT_DIR], 0),
            PORT_OUT..=PORT_OUTTGL => (self.regs[PORT_OUT], 0),
            PORT_IN => (self.regs[PORT_IN], 0), //TODO: update reg value on pin status change
            PORT_INTFLAGS => (self.regs[PORT_INTFLAGS], 0),
            PORT_PORTCTRL => (self.regs[PORT_PORTCTRL] & 0x01, 0),
            PORT_PIN0CTRL..=PORT_PIN7CTRL => (self.regs[address] & 0x8F, 0),
            _ => panic!("Attempt to access invalid register in PORT peripheral.")
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
            _ => panic!("Attenmt to access invalid register in PORT peripheral.")
        }
        0
    }
}

impl Hardware for Port {
    fn update(&mut self, _time: usize) {
        for i in 0..8 {
            match self.pio[i].net.borrow().state {
                NetState::High => self.regs[PORT_IN] |= 1u8 << i,
                NetState::Low => self.regs[PORT_IN] &= !(1u8 << i),
                _ => {} //do nothing if undefined
            }
        }
    }
}

pub struct VirtualPort {
    pub port: Rc<RefCell<Port>>
}

impl MemoryMapped for VirtualPort {
    fn get_size(&self) -> usize {
        4
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        self.port.borrow_mut().read(
            match address {
                0x00 => 0x00,
                0x01 => 0x04,
                0x02 => 0x08,
                0x03 => 0x09,
                _ => 0x0B
            }
        )    
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        self.port.borrow_mut().write(
            match address {
                0x00 => 0x00,
                0x01 => 0x04,
                0x02 => 0x08,
                0x03 => 0x09,
                _ => 0x0B
            }, 
            value
        ) 
    }
}

