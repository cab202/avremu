pub mod port;

pub trait Clocked {
    fn tick();
}

