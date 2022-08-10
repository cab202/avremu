use std::rc::Rc;
use std::cell::RefCell;

use super::Hardware;
use crate::nets::{PinState, Net};


#[derive(Debug)]
#[derive(PartialEq)]
enum PushbuttonState {
    Released, 
    Pressed
}

pub struct Pushbutton {
    name: String,
    pin: Rc<RefCell<PinState>>,
    //net: Rc<RefCell<Net>>,
    state: PushbuttonState,
    active_high: bool
}

impl Pushbutton {
    pub fn new(name: String, active_high: bool, net: Rc<RefCell<Net>>) -> Self {
        let pb = Pushbutton { 
            name,
            pin: Rc::new(RefCell::new(PinState::Open)),
            //net,
            state: PushbuttonState::Released,
            active_high
        };
        net.borrow_mut().connect(Rc::downgrade(&pb.pin));
        pb
    }

    pub fn press(&mut self, time: usize) {
        if !self.state.eq(&PushbuttonState::Pressed) {
            println!("[@{:08X}] PB|{}: {}", time, self.name, "Pressed");
        }
        
        self.state = PushbuttonState::Pressed;
        if self.active_high {
            *self.pin.borrow_mut() = PinState::DriveH;
        } else {
            *self.pin.borrow_mut() = PinState::DriveL;
        }
    }

    pub fn release(&mut self, time: usize) {
        if !self.state.eq(&PushbuttonState::Released) {
            println!("[@{:08X}] PB|{}: {}", time, self.name, "Released");
        }
        
        self.state = PushbuttonState::Released;
        *self.pin.borrow_mut() = PinState::Open;
    }
}

impl Hardware for Pushbutton {
    fn update(&mut self, _time: usize) {
        //TODO: Add code for bounce
    }

    fn event(&mut self, time: usize, event: &String) {
        if event.eq("PRESS") {
            self.press(time);
        } else if event.eq("RELEASE") {
            self.release(time);
        }
    }

    //fn get_pin(&self, _name: String) -> Weak<RefCell<PinState>> {
    //    Rc::downgrade(&self.pin)
    //}
}