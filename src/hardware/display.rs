use std::rc::Rc;
use std::cell::RefCell;

use super::Hardware;
use crate::nets::{PinState, Net, NetState};

pub struct Display {
    name: String,
    pins_segs: [Rc<RefCell<PinState>>; 7],
    pin_en: Rc<RefCell<PinState>>,
    pin_digit: Rc<RefCell<PinState>>,
    nets_segs: [Rc<RefCell<Net>>; 7],
    net_en: Rc<RefCell<Net>>,
    net_digit: Rc<RefCell<Net>>,
    enabled: bool,
    state: u8
}

impl Display {
    pub fn new(name: String) -> Self {
        Display { 
            name,
            pins_segs: [
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp)),
                Rc::new(RefCell::new(PinState::WeakPullUp))
            ],
            pin_en: Rc::new(RefCell::new(PinState::WeakPullUp)),
            pin_digit: Rc::new(RefCell::new(PinState::WeakPullDown)),
            nets_segs: [
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string())))
            ],
            net_en: Rc::new(RefCell::new(Net::new("".to_string()))),
            net_digit: Rc::new(RefCell::new(Net::new("".to_string()))),
            enabled: true, 
            state: 0
        }
    }

    pub fn connect_seg(&mut self, n: usize, net: Rc<RefCell<Net>>) {
        self.nets_segs[n] = net;
        self.nets_segs[n].borrow_mut().connect(Rc::downgrade(&self.pins_segs[n]));
    }

    pub fn connect(&mut self, pin_name: &str, net: Rc<RefCell<Net>>) {
        match pin_name {
            "en" => {self.net_en = net; self.net_en.borrow_mut().connect(Rc::downgrade(&self.pin_en))},
            "digit" => {self.net_digit = net; self.net_digit.borrow_mut().connect(Rc::downgrade(&self.pin_digit))},
             _ => {}
        }
    }

    fn decode(&self) -> String {
        let code = self.state & 0x7F;
        let symbol = match code {
            0b01111111 => "Off".to_string(),
            0b00001000 => "0".to_string(),
            0b01101011 => "1".to_string(),
            0b00111110 => "1'".to_string(),
            0b01000100 => "2".to_string(),
            0b01000001 => "3".to_string(),
            0b00100011 => "4".to_string(),
            0b00010001 => "5".to_string(),
            0b00010000 => "6".to_string(),
            0b01001011 => "7".to_string(),
            0b00000000 => "8".to_string(),
            0b00000001 => "9".to_string(),
            0b00000010 => "A".to_string(),
            0b00110000 => "B".to_string(),
            0b00011100 => "C".to_string(),
            0b01100000 => "D".to_string(),
            0b00010100 => "E".to_string(),
            0b00010110 => "F".to_string(),
            0b01110111 => "-".to_string(),
            _ => format!("0x{:02X}", code)
        };
        let digit = if (self.state & 0x80) == 0x00 {"RHS"} else {"LHS"};
        format!("{} ({})", symbol, digit)
    }
}

impl Hardware for Display {
    fn update(&mut self, time: usize) {
        self.enabled = self.net_en.borrow().state.eq(&NetState::High);
        //println!("DISP: Enable is {}", self.enabled);
        let mut state_new = 0x7F;
        if self.enabled {
            for i in 0..7 {
                if self.nets_segs[i].borrow().state.eq(&NetState::Low) {
                   //println!("DISP: Seg {} is low.", i);
                    state_new &= !(1 << i)
                }
            }
        }
        match self.net_digit.borrow().state {
            NetState::Low => state_new &= 0x7F,
            NetState::High => state_new |= 0x80,
            _ => {}
        }
        
        let mut print_state = false;
        if self.state != state_new {
            if (self.state & state_new) & 0x7F != 0x7F {
                print_state = true;
            }
        }

        self.state = state_new;
    
        if print_state {
            println!("[@{:08X}] DISP|{}: {}", time, self.name, self.decode());
        }

        //println!("DISP: State {} => {}", self.state, state_new);
        
    }
}