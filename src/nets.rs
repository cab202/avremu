use std::rc::Weak;
use std::cell::RefCell;

use crate::CLI;

#[derive(Debug)]
pub enum PinState {
    Open,
    WeakPullDown,
    WeakPullUp,
    UndefinedWeak,
    DriveL,
    DriveH,
    DriveAnalog(f32),
    UndefinedStrong
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone, Copy)]
pub enum NetState {
    Undefined,
    High,
    Low,
    Analog(f32)
}

#[allow(dead_code)]
pub struct Net {
    pub state: NetState,
    io: Vec<Weak<RefCell<PinState>>>,
    name: String
}

impl Net {
    pub fn new(name: String) -> Self {
        Net { 
            state: NetState::Undefined, 
            io: Vec::new(), 
            name
        }
    }

    pub fn connect(&mut self, pin: Weak<RefCell<PinState>>) {
        self.io.push(pin);
    }

    pub fn update(&mut self, time: u64) {
        let mut dps = PinState::Open;

        //println!("Updating net...");

        for ps in &self.io {

            //println!("Pinstate is {:?}", *ps.upgrade().unwrap().borrow());

            match *ps.upgrade().unwrap().borrow() {
                PinState::Open => {},
                PinState::WeakPullDown => {
                    match dps {
                        PinState::Open => dps = PinState::WeakPullDown,
                        PinState::WeakPullUp => dps = PinState::UndefinedWeak,
                        _ => {} // if driven, already pulled down, or undefined, state unchanged
                    }
                },
                PinState::WeakPullUp => {
                    match dps {
                        PinState::Open => dps = PinState::WeakPullUp,
                        PinState::WeakPullDown => dps = PinState::UndefinedWeak,
                        _ => {} // if driven, already pulled up, or undefined, state unchanged
                    }
                },
                PinState::UndefinedWeak => {
                    match dps {
                        PinState::DriveH => dps = PinState::DriveH,
                        PinState::DriveL => dps = PinState::DriveL,
                        PinState::DriveAnalog(v) => dps = PinState::DriveAnalog(v),
                        _ => {} // unless driven, remains weakly undefined
                    }
                },
                PinState::DriveL => {
                    match dps {
                        PinState::DriveH | PinState::DriveAnalog(_) => {dps = PinState::UndefinedStrong; break}, // The net cant get out of this state
                        _ => dps = PinState::DriveL
                    }
                },
                PinState::DriveH => {
                    match dps {
                        PinState::DriveL | PinState::DriveAnalog(_) => {dps = PinState::UndefinedStrong; break}, // The net cant get out of this state
                        _ => dps = PinState::DriveH
                    }
                },
                PinState::DriveAnalog(v) => {
                    match dps {
                        PinState::DriveL | PinState::DriveH | PinState::DriveAnalog(_)=> {dps = PinState::UndefinedStrong; break}, // The net cant get out of this state
                        _ => dps = PinState::DriveAnalog(v)
                    }
                },
                PinState::UndefinedStrong => break  // We shouldn't ever get here
            }

            //println!("New dominant pinstate is {:?}", dps);
        }

        let state_new;
        match dps {
            PinState::Open | PinState::UndefinedWeak |  PinState::UndefinedStrong => state_new = NetState::Undefined,
            PinState::WeakPullDown | PinState::DriveL => state_new = NetState::Low,
            PinState::WeakPullUp | PinState::DriveH => state_new = NetState::High,
            PinState::DriveAnalog(v) => state_new = NetState::Analog(v),      
        }

        if self.state != state_new {
            if CLI.net_all | (CLI.net_undef & self.state.eq(&NetState::Undefined)) {
                if time > 0 {
                    println!("[@{:012X}] NET|{}: {:?} => {:?}", time, self.name, self.state, state_new)
                };
            }
        }
        self.state = state_new;

    }
}