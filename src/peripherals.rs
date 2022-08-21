pub mod port;
pub mod spi;
pub mod stdio;

pub trait Clocked {
    fn tick(&mut self, time: usize);
}

