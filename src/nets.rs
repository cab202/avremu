use std::rc::Rc;

pub enum PinState {
    Open,
    DriveL,
    DriveH,
    DriveAnalog(f32),
    WeakPullDown,
    WeakPullUp
}

pub enum NetState {
    High,
    Low,
    Analog(f32)
}

pub struct Net {
    state: NetState,
    io: Vec<Rc<PinState>>
}