use std::rc::Rc;
use std::cell::RefCell;

use super::Hardware;
use crate::nets::{PinState, Net, NetState};


#[derive(Debug)]
#[derive(PartialEq)]
enum BuzzerState {
    High, 
    Low,
    Undefined
}

pub struct Buzzer {
    name: String,
    pin: Rc<RefCell<PinState>>,
    net: Rc<RefCell<Net>>,
    state: BuzzerState,
    t_rise_last: u64,
    t_fall_last: u64,
    t_last: u64,
    cycle_valid: bool,
    is_dc: bool,
    desc: String
}

impl Buzzer {
    pub fn new(name: String, net: Rc<RefCell<Net>>) -> Self {
        let mut buzzer = Buzzer { 
            name,
            pin: Rc::new(RefCell::new(PinState::Open)),
            net,
            state: BuzzerState::Undefined,
            t_rise_last: 0,
            t_fall_last: 0,
            t_last: 0,
            cycle_valid: false,
            is_dc: false,
            desc: String::new()
        };
        buzzer.net.borrow_mut().connect(Rc::downgrade(&buzzer.pin));
        buzzer.update(0);
        buzzer
    }
}

impl Hardware for Buzzer {
    fn update(&mut self, time: u64) {
        let new_state: BuzzerState;
        match self.net.borrow().state {
            NetState::High => new_state = BuzzerState::High,
            NetState::Low => new_state = BuzzerState::Low,
            _ => new_state = BuzzerState::Undefined
        }
        if !self.state.eq(&new_state) {
            self.t_last = time;
            self.is_dc = false;
        } else {
            if time - self.t_last > 50000000 {
                // Greater than 50ms elapsed, assume DC
                if !self.is_dc {
                    self.is_dc = true;
                    match self.state {
                        BuzzerState::Low | BuzzerState::Undefined => println!("[@{:012X}] BUZZER|{}: {:.1} Hz, {:.1} % duty cycle", self.t_last, self.name, 0.0, 0.0),
                        BuzzerState::High => println!("[@{:012X}] BUZZER|{}: {:.1} Hz, {:.1} % duty cycle", self.t_last, self.name, 0.0, 100.0)
                    }
                }
            }
        }
        match self.state {
            BuzzerState::Low => {
                if new_state.eq(&BuzzerState::High) {
                    let diff = time - self.t_rise_last;
                    let ppw = self.t_fall_last - self.t_rise_last;
                    let f = 1e9/f64::from(diff as i32);
                    let duty = 100.0*f64::from(ppw as i32)/f64::from(diff as i32);
                    if self.cycle_valid {
                        let desc_new = format!("{:.1} Hz, {:.1} % duty cycle", f, duty);
                        if self.desc.ne(&desc_new) {
                            println!("[@{:012X}] BUZZER|{}: {}", time, self.name, desc_new);
                        }
                        self.desc = desc_new;
                    }
                    //if self.cycle_valid & (f > 20.0) & (f < 20000.0) {
                    //    println!("[@{:012X}] BUZZER|{}: {:.1} Hz, {:.1} % duty cycle", time, self.name, f, duty);
                    //}
                    self.t_rise_last = time;
                    if !self.cycle_valid {
                        self.cycle_valid = true;
                    }
                }
                
            },
            BuzzerState::High => {
                if new_state.eq(&BuzzerState::Low) {
                    self.t_fall_last = time
                }
            },
            _ => {self.cycle_valid = false}
        }
        self.state = new_state;
    }

    //fn get_pin(&self, _name: String) -> Weak<RefCell<PinState>> {
    //    Rc::downgrade(&self.pin)
    //}
}