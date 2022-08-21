use std::collections::VecDeque;
use std::fs;

use crate::memory::MemoryMapped;
use crate::hardware::Hardware;

const STDIO_OUT:    usize = 0x00;
const STDIO_IN:     usize = 0x01;
const STDIO_AVAIL:  usize = 0x02;

pub struct Stdio {
    name: String,
    out: String,
    outfile: String,
    input: VecDeque<u8>
}

impl Stdio {
    pub fn new(name: String, outfile: String) -> Self {
        Stdio {
            name,
            out: "".to_string(),
            outfile, 
            input: VecDeque::new()
        }
    }

    fn out(&mut self, c: u8) {
        self.out.push(c as char);
        //println!("[STDIO] Wrote 0x{:02X} ({})", c as u8, c);
    }

    pub fn out_close(&self) {
        fs::write(&self.outfile, &self.out).expect(&format!("Unable to write stdout to {}.", self.outfile));
    }
}

impl MemoryMapped for Stdio {
    fn get_size(&self) -> usize {
        3
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            STDIO_IN => {(self.input.pop_front().unwrap_or_else(|| 0u8), 0)},
            STDIO_AVAIL => {(self.input.len() as u8, 0)},
            _ => {(0, 0)}
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            STDIO_OUT => self.out(value),
            _ => {}
        }
        0
    }
}

impl Hardware for Stdio {
    fn update(&mut self, _time: usize) {

    }

    fn event(&mut self, time: usize, event: &String) {
        //TODO: Handle keystrokes
    }
}