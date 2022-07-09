use super::cores::Core;
use super::memory::Memory;

pub mod devices {
    pub enum DeviceType {
        ATtiny1626
    }

    struct Device {
        core: Core,
        flash: Memory,
        sram: Memory,
        mm: Vec<Rc<RefCell<MemoryMapped>>>
    }

    impl Device {
        fn new(dt: DeviceType) -> Self {
            match dt {
                ATtiny1626 => Device {
                    core: Core::new(),
                    flash: Memory::new(0, 8192, 0, 0xFF),
                    sram: Memory::new(0, 256, 0, 0x00),
                    mm: vec!(
                        Rc::new(RefCell::new(self.flash)),
                        Rc::new(RefCell::new(self.sram)),
                    )
                }
            }
        }
    }
}