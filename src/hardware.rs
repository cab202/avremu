pub mod display;
pub mod ic74hc595;
pub mod led;
pub mod pot;
pub mod pushbutton;
pub mod sinkpwm;
pub mod sinkuart;

pub trait Hardware {
    fn update(&mut self, time: u64);
    fn event(&mut self, _time: u64, _event: &str) {}
}
