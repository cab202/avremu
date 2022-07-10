mod devices;
mod cores;
mod memory;

use crate::devices::Device;
use crate::devices::DeviceType;

fn main() {
    let mut a: i8 = 0x01;
    let _b: u8 = 0xFF;

    for i in 0..16 {
        println!("{}: a = {}", i, a);
        a <<= 1;
    }

    let mut mcu = Device::new(DeviceType::ATtiny1626);

//    let mut core = Core::new();
//    core.set_r(24,0xFF);
//    core.set_r(25,0x01);

//    core.spm(0,0b1001_0110_0000_0001); //ADIW(R24, 0x01)
//    core.spm(1,0x0000);
//    core.spm(2,0x0000);
//    core.spm(3, 0b1001_0101_1001_1000);

//    while core.tick() {
        // Run until break
//    }

//    for i in 0..=31 {
//        println!("r{:02} = 0x{:02X}", i, core.get_r(i));
//    }
    
}
