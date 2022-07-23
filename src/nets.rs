use std::rc::Rc;

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

pub enum NetState {
    Undefined,
    High,
    Low,
    Analog(f32)
}

pub struct Net {
    state: NetState,
    io: Vec<Rc<PinState>>
}

impl Net {
    fn update(&mut self) {
        let mut dps = PinState::Open;

        for ps in &self.io {
            match **ps {
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
        }

        match dps {
            PinState::Open | PinState::UndefinedWeak |  PinState::UndefinedStrong => self.state = NetState::Undefined,
            PinState::WeakPullDown | PinState::DriveL => self.state = NetState::Low,
            PinState::WeakPullUp | PinState::DriveH => self.state = NetState::High,
            PinState::DriveAnalog(v) => self.state = NetState::Analog(v),      
        }
    }
}