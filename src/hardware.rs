pub mod pushbutton;
pub mod led;
pub mod ic74hc595;
pub mod display;
pub mod sinkpwm;
pub mod pot;
pub mod sinkuart;

pub trait Hardware {
    fn update(&mut self, time: u64);
    fn event(&mut self, _time: u64, _event: &String) { }
    //fn get_pin(&self, name: String) -> Weak<RefCell<PinState>>;
}