use std::cell::RefCell;
use std::rc::Rc;

pub trait MemoryMapped {
    fn get_size(&self) -> usize;
    fn read(&self, address: usize) -> (u8, usize);
    fn read_word(&self, address: usize) -> (u16, usize);
    fn write(&mut self, address: usize, value: u8) -> usize;
    fn write_word(&mut self, address: usize, value: u16) -> usize;
}

pub struct MemoryMap {
    size: usize,
    mm: Vec<Rc<RefCell<dyn MemoryMapped>>>

}

impl MemoryMap {
    pub fn new(size: usize) -> Self {
        MemoryMap { size, mm: Vec::new() }
    }

    pub fn add(&mut self, offset: usize, dev: Rc<RefCell<dyn MemoryMapped>>) {
        self.mm.push(dev);
    }
}

impl MemoryMapped for MemoryMap {
    fn get_size(&self) -> usize {
        self.mm.len()
    }

    fn read(&self, address: usize) -> (u8, usize) {
        (0, 0)  //TODO
    }

    fn read_word(&self, address: usize) -> (u16, usize) {
        (0, 0) //TODO
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        0 //TODO
    }

    fn write_word(&mut self, address: usize, value: u16) -> usize {
        0 //TODO
    }
}



pub struct Memory {
    mem: Vec<u8>,
    lat: usize
}

impl Memory {
    pub fn new(size: usize, fill: u8, lat: usize) -> Self {
        Memory {
            mem: vec![fill; size],
            lat
        }
    }
}

impl MemoryMapped for Memory {
    fn get_size(&self) -> usize {
        self.mem.len()
    }

    fn read(&self, address: usize) -> (u8, usize) {
        (self.mem[address], self.lat)
    }

    fn read_word(&self, address: usize) -> (u16, usize) {
        let mut word = (self.mem[address+1] as u16) << 8;
        word |= self.mem[address] as u16;
        (word, self.lat)
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        self.mem[address] = value;
        0
    }

    fn write_word(&mut self, address: usize, value: u16) -> usize {
        self.mem[address] = value as u8;
        self.mem[address+1] = (value >> 8) as u8;
        0
    }
}