use std::rc::Rc;
use std::cell::RefCell;

use super::Hardware;
use crate::nets::{PinState, Net};

pub struct Pot {
    name: String,
    pin: Rc<RefCell<PinState>>
}

impl Pot {
    pub fn new(name: String, net: Rc<RefCell<Net>>, position: f32) -> Self {
        let mut pot = Pot { 
            name,
            pin: Rc::new(RefCell::new(PinState::DriveAnalog(0.0)))
        };
        net.borrow_mut().connect(Rc::downgrade(&pot.pin));
        pot.set(0, position);
        pot
    }

    fn set(&mut self, time: u64, position: f32) {
        
        let pos = position.min(1.0).max(0.0);

        println!("[@{:08X}] POT|{}: {:.3}", time, self.name, pos);
        
        *self.pin.borrow_mut() = PinState::DriveAnalog(3.3*pos);
    }
}

impl Hardware for Pot {

    fn update(&mut self, _time: u64) {
        
    }

    fn event(&mut self, time: u64, event: &String) {
        let pos: f32 = event.parse().unwrap();
        self.set(time, pos);
    }
}