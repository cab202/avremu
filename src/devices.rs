use super::cores::Core;
use super::cores::CoreType;
use super::memory::Memory;
use super::memory::MemoryMap;
use super::memory::MemoryMapped;

use std::cell::RefCell;
use std::rc::Rc;

pub enum DeviceType {
    ATtiny1626
}

pub struct Device {
    core: Core,
    flash: Rc<RefCell<dyn MemoryMapped>>,
    sram: Rc<RefCell<dyn MemoryMapped>>,
    mm: MemoryMap
}

impl Device {
    pub fn new(dt: DeviceType) -> Self {
        match dt {
            DeviceType::ATtiny1626 => {
                let flash: Rc<RefCell<dyn MemoryMapped>> =  Rc::new(RefCell::new(Memory::new(16384, 0xFF, 0)));
                let sram: Rc<RefCell<dyn MemoryMapped>> = Rc::new(RefCell::new(Memory::new(256, 0, 0)));

                let mut mm = MemoryMap::new(32768);
                mm.add(0x0000, Rc::clone(&sram));
                mm.add(0x1000, Rc::clone(&flash));

                Device {
                    core: Core::new(CoreType::AVRxt, Rc::clone(&sram), Rc::clone(&flash)),
                    flash: flash,
                    sram: sram,
                    mm
                }
            }
            
            
        }
    }
}