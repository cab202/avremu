use crate::nets::{PinState, Net};

enum PushbuttonType {
    SPST
}
struct Pushbutton {
    config: PushbuttonType,
    pins: Vec<Net>,
    //bouncy: bool
}

impl Pushbutton {
    fn new(config: PushbuttonType, pins: Vec<Net>) -> Self {
        Pushbutton {
            config,
            pins
        }
    }
}