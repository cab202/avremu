use std::cell::RefCell;
use std::rc::Rc;

use super::Hardware;
use crate::nets::{Net, PinState};

#[derive(Debug, PartialEq)]
enum PushbuttonState {
    Released,
    Pressed,
}

pub struct Pushbutton {
    name: String,
    pin: Rc<RefCell<PinState>>,
    //net: Rc<RefCell<Net>>,
    state: PushbuttonState,
    active_high: bool,
}

impl Pushbutton {
    pub fn new(name: String, active_high: bool, net: Rc<RefCell<Net>>) -> Self {
        let pb = Pushbutton {
            name,
            pin: Rc::new(RefCell::new(PinState::Open)),
            //net,
            state: PushbuttonState::Released,
            active_high,
        };
        net.borrow_mut().connect(Rc::downgrade(&pb.pin));
        pb
    }

    pub fn press(&mut self, time: u64) {
        if !self.state.eq(&PushbuttonState::Pressed) && time > 0 {
            println!("[@{:012X}] PB|{}: Pressed", time, self.name);
        }

        self.state = PushbuttonState::Pressed;
        if self.active_high {
            *self.pin.borrow_mut() = PinState::DriveH;
        } else {
            *self.pin.borrow_mut() = PinState::DriveL;
        }
    }

    pub fn release(&mut self, time: u64) {
        if !self.state.eq(&PushbuttonState::Released) && time > 0 {
            println!("[@{:012X}] PB|{}: Released", time, self.name);
        }

        self.state = PushbuttonState::Released;
        *self.pin.borrow_mut() = PinState::Open;
    }
}

impl Hardware for Pushbutton {
    fn update(&mut self, _time: u64) {
        //TODO: Add code for bounce
    }

    fn event(&mut self, time: u64, event: &str) {
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
