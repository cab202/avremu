use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::nets::PinState;

mod resistor;
mod pushbutton;
pub mod led;

pub trait Hardware {
    fn update(&mut self, time: usize);
    fn get_pin(&self, name: String) -> Weak<RefCell<PinState>>;
}