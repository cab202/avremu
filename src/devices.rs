use super::cores::Core;
use super::cores::CoreType;
use super::memory::Memory;
use super::memory::MemoryMap;
use super::memory::MemoryMapped;

use std::cell::RefCell;
use std::rc::Rc;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use ihex::Reader;
use ihex::Record;

pub enum DeviceType {
    ATtiny1626
}

pub struct Device {
    pub core: Core,
    pub flash: Rc<RefCell<dyn MemoryMapped>>,
    pub sram: Rc<RefCell<dyn MemoryMapped>>,
    pub mm: MemoryMap
}

impl Device {
    pub fn new(dt: DeviceType) -> Self {
        match dt {
            DeviceType::ATtiny1626 => {
                let flash: Rc<RefCell<dyn MemoryMapped>> =  Rc::new(RefCell::new(Memory::new(16384, 0xFF, 0)));
                let sram: Rc<RefCell<dyn MemoryMapped>> = Rc::new(RefCell::new(Memory::new(2048, 0x00, 0)));

                let mut mm = MemoryMap::new();
                mm.add(0x3400, Rc::clone(&sram));
                mm.add(0x8000, Rc::clone(&flash));

                Device {
                    core: Core::new(CoreType::AVRxt, Rc::clone(&sram), Rc::clone(&flash)),
                    flash: flash,
                    sram: sram,
                    mm
                }
            }
            
            
        }
    }

    pub fn load_hex(&mut self, filename: &String) {
        let path = Path::new(filename);
        let display = path.display();
    
        // Open the path in read-only mode, returns `io::Result<File>`
        let mut file = match File::open(&path) {
            Err(why) => panic!("Couldn't open {}: {}", display, why),
            Ok(file) => file,
        };
    
        // Read the file contents into a string, returns `io::Result<usize>`
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => panic!("Couldn't read {}: {}", display, why),
            Ok(_) => {
                let hex = Reader::new(&s);
                for r in hex {
                    if let Record::Data{offset, value} = r.unwrap() {
                        println!("[HEX] 0x{:04X} Writing {} bytes.", offset, value.len());
                        let mut address = usize::from(offset);
                        for b in value {
                            self.flash.borrow_mut().write(address, b);
                            address += 1;
                        }
                    }
                }
            }
        };

    }

    pub fn load_test_programme(&mut self) {
        self.core.set_r(24, 0xFF);
        self.core.set_r(25, 0x01);

        self.flash.borrow_mut().write_word(0 <<1, 0b1001_0110_0000_0001);
        self.flash.borrow_mut().write_word(1 <<1, 0x0000);
        self.flash.borrow_mut().write_word(2 <<1, 0x0000);
        self.flash.borrow_mut().write_word(3 <<1, 0b1001_0101_1001_1000);
    }

    pub fn tick(&mut self) -> bool {
        self.core.tick()
    }

    pub fn dump_regs(&self) {
        for i in 0..=31 {
            println!("r{:02} = 0x{:02X}", i, self.core.get_r(i));
        }
    }
}