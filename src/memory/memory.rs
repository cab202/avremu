pub mod memory {
    pub trait MemoryMapped {
        fn get_offet(&self) -> u32;
        fn get_size(&self) -> u32;
        fn read(&self, address: u32) -> (u8, u32);
        fn write(&mut self, address: u32, value: u8) -> u32;
    }

    pub struct MemoryMap<T: MemoryMapped> {
        mm: Vec<Rc<RefCell<T>>>
    }

    pub struct Memory {
        offset: u32,
        size: u32,
        lat: u32,
        mem: Box<[u8]>
    }

    impl Memory {
        fn new(offset: u32, size: u32, lat: u8, fill: u8) {
            Memory {
                offset,
                size,
                lat,
                mem: Box::new([u8; fill])
            }
        }
    }

    impl MemoryMapped for Memory {
        fn get_offet(&self) -> u32 {
            self.offset
        }

        fn get_size(&self) -> u32 {
            slef.size
        }

        fn read(&self, address: u32) -> (u8, u32) {
            (*(self.mem)[usize::try_from(address-self.offset).unwrap()], 0)
        }

        fn write(&mut self, address: u32, value: u8) -> u32 {
            *(self.mem)[usize::try_from(address-self.offset).unwrap()] = value;
            0
        }
    }
}