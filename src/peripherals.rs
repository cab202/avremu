pub mod port;
pub mod spi;

pub trait Clocked {
    fn tick(&mut self, time: usize);
}

