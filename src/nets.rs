use std::rc::{Rc, Weak};
use std::cell::RefCell;

pub type Pin = Rc<PinState>;

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
pub enum NetState {
    Undefined,
    High,
    Low,
    Analog(f32)
}

pub struct Net {
    pub state: NetState,
    io: Vec<Weak<RefCell<PinState>>>
}

impl Net {
    pub fn new() -> Self {
        Net { 
            state: NetState::Undefined, 
            io: Vec::new()
        }
    }

    pub fn connect(&mut self, pin: Weak<RefCell<PinState>>) {
        self.io.push(pin);
    }

    pub fn update(&mut self) {
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

        match dps {
            PinState::Open | PinState::UndefinedWeak |  PinState::UndefinedStrong => self.state = NetState::Undefined,
            PinState::WeakPullDown | PinState::DriveL => self.state = NetState::Low,
            PinState::WeakPullUp | PinState::DriveH => self.state = NetState::High,
            PinState::DriveAnalog(v) => self.state = NetState::Analog(v),      
        }

        println!("New net state is {:?}", self.state);


    }
}