use std::rc::Rc;
use std::cell::RefCell;

use super::Hardware;
use crate::nets::{PinState, Net, NetState};


#[derive(Debug)]
#[derive(PartialEq)]
enum LedState {
    On, 
    Off,
    Undefined
}

pub struct Led {
    name: String,
    pin: Rc<RefCell<PinState>>,
    net: Rc<RefCell<Net>>,
    state: LedState,
    active_high: bool
}

impl Led {
    pub fn new(name: String, active_high: bool, net: Rc<RefCell<Net>>) -> Self {
        let mut led = Led { 
            name,
            pin: if active_high {
                Rc::new(RefCell::new(PinState::WeakPullDown))
            } else {
                Rc::new(RefCell::new(PinState::WeakPullUp))
            },
            net,
            state: LedState::Undefined,
            active_high
        };
        led.net.borrow_mut().connect(Rc::downgrade(&led.pin));
        led.update(0);
        led
    }
}

impl Hardware for Led {
    fn update(&mut self, time: u64) {
        let new_state: LedState;
        match self.net.borrow().state {
            NetState::High => if self.active_high {new_state = LedState::On} else {new_state = LedState::Off},
            NetState::Low => if !self.active_high {new_state = LedState::On} else {new_state = LedState::Off},
            _ => new_state = LedState::Undefined
        }
        if !self.state.eq(&new_state) {
            println!("[@{:012X}] LED|{}: {:?}", time, self.name, new_state);
            self.state = new_state;
        } 
    }

    //fn get_pin(&self, _name: String) -> Weak<RefCell<PinState>> {
    //    Rc::downgrade(&self.pin)
    //}
}