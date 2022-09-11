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
    t_rise_last: usize,
    t_fall_last: usize,
    cycle_valid: bool
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
            cycle_valid: false
        };
        buzzer.net.borrow_mut().connect(Rc::downgrade(&buzzer.pin));
        buzzer.update(0);
        buzzer
    }
}

impl Hardware for Buzzer {
    fn update(&mut self, time: usize) {
        let new_state: BuzzerState;
        match self.net.borrow().state {
            NetState::High => new_state = BuzzerState::High,
            NetState::Low => new_state = BuzzerState::Low,
            _ => new_state = BuzzerState::Undefined
        }
        match self.state {
            BuzzerState::Low => {
                if new_state.eq(&BuzzerState::High) {
                    let diff = time - self.t_rise_last;
                    let ppw = self.t_fall_last - self.t_rise_last;
                    let f = 3333333.333/f64::from(diff as i32);
                    let duty = f64::from(ppw as i32)/f64::from(diff as i32);
                    if self.cycle_valid & (f > 20.0) & (f < 20000.0) {
                        println!("[@{:08X}] BUZZER|{}: {:.0} Hz, {:.1} % duty cycle", time, self.name, f, duty);
                    }
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