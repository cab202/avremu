use std::collections::VecDeque;
use std::fs;

use crate::hardware::Hardware;
use crate::memory::MemoryMapped;

const STDIO_OUT: usize = 0x00;
const STDIO_IN: usize = 0x01;
const STDIO_AVAIL: usize = 0x02;

#[allow(dead_code)]
pub struct Stdio {
    name: String,
    out: Vec<u8>,
    outfile: String,
    input: VecDeque<u8>,
}

impl Stdio {
    pub fn new(name: String, outfile: String) -> Self {
        Stdio {
            name,
            out: Vec::new(),
            outfile,
            input: VecDeque::new(),
        }
    }

    fn out(&mut self, c: u8) {
        self.out.push(c);
    }

    pub fn out_close(&self) {
        fs::write(&self.outfile, &self.out)
            .unwrap_or_else(|_| panic!("Unable to write stdout to {}.", self.outfile));
    }
}

impl MemoryMapped for Stdio {
    fn get_size(&self) -> usize {
        3
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            STDIO_IN => (self.input.pop_front().unwrap_or_default(), 0),
            STDIO_AVAIL => (self.input.len() as u8, 0),
            _ => (0, 0),
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        if let STDIO_OUT = address {
            self.out(value)
        }
        0
    }
}

impl Hardware for Stdio {
    fn update(&mut self, _time: u64) {}

    fn event(&mut self, _time: u64, _event: &str) {
        // TODO: Handle keystrokes
    }
}
