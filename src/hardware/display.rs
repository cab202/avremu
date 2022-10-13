use std::{rc::Rc, collections::VecDeque};
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
    state: VecDeque<(u8, usize)>,
    state_2d: String
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
            state: VecDeque::from(vec!((0,0), (0,0), (0,0))),
            state_2d: "".to_string()
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

    fn seg_to_char(segs: u8) -> String {
        match segs & 0x7F {
            0b01111111 => " ".to_string(),
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
            _ => "?".to_string()
        }
    }

    fn decode(&self) -> String {
        let code = self.state.front().unwrap().0 & 0x7F;
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
        let digit = if (self.state.front().unwrap().0 & 0x80) == 0x00 {"RHS"} else {"LHS"};
        format!("{} ({})", symbol, digit)
    }

    fn decode_2d(&self) -> String {
        let lhs_first = (self.state.front().unwrap().0 & 0x80) != 0;
        let lhs;
        let rhs;
        if lhs_first {
            //LHS first
            lhs = self.state.front().unwrap();
            rhs = self.state.get(1).unwrap();
        } else {
            lhs = self.state.get(1).unwrap();
            rhs = self.state.front().unwrap();
        }

        let mut disp = String::new();
        disp.push_str(&Self::seg_to_char(lhs.0));
        disp.push_str(&Self::seg_to_char(rhs.0));

        let time0 = self.state.get(0).unwrap().1;
        let time1 = self.state.get(1).unwrap().1;
        let time2 = self.state.get(2).unwrap().1;

        let period = time0-time2;
        let inton;
        if lhs_first {
            inton = time1-time2; 
        } else {
            inton = time0-time1; 
        }

        let freq = 3333333.3/(period as f64);
        let duty = 100.0*(inton as f64)/(period as f64);
                
        format!("{} ({:.0} Hz, {:.0} %)", disp, freq, duty)
    }
}

impl Hardware for Display {
    fn update(&mut self, time: usize) {
        self.enabled = self.net_en.borrow().state.eq(&NetState::High);
        //println!("DISP: Enable is {}", self.enabled);
        let state = self.state.front().unwrap().0;
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
        if state != state_new {
            if (state & state_new) & 0x7F != 0x7F {
                print_state = true;
            }

            self.state.push_front((state_new, time));
            self.state.pop_back();

            let valid_2d_cycle = self.state.front().unwrap().0 == self.state.back().unwrap().0;
            //println!("[DISP] Front: {:02X}, Back: {:02X}", self.state.front().unwrap().0, self.state.back().unwrap().0);
            
            if valid_2d_cycle {
                let state_2d_new = self.decode_2d();
                if self.state_2d.ne(&state_2d_new) {
                    self.state_2d = state_2d_new;
                    println!("[@{:08X}] DISP|{}: {}", time, self.name, self.state_2d);
                }
            } else if print_state {
                println!("[@{:08X}] DISP|{}: {}", time, self.name, self.decode());
            }
        }  

        //println!("DISP: State {} => {}", self.state, state_new);
        
    }
}