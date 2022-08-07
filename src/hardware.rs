use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::nets::PinState;

mod resistor;
pub mod pushbutton;
pub mod led;
pub mod buzzer;

pub trait Hardware {
    fn update(&mut self, time: usize);
    fn event(&mut self, time: usize, event: &String) {

    }
    //fn get_pin(&self, name: String) -> Weak<RefCell<PinState>>;
}