mod resistor;
pub mod pushbutton;
pub mod led;
pub mod buzzer;
pub mod ic74hc595;
pub mod display;
pub mod sinkpwm;

pub trait Hardware {
    fn update(&mut self, time: usize);
    fn event(&mut self, _time: usize, _event: &String) { }
    //fn get_pin(&self, name: String) -> Weak<RefCell<PinState>>;
}