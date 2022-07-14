mod devices;
mod cores;
mod memory;

use crate::devices::Device;
use crate::devices::DeviceType;

fn main() {
    let mut mcu = Device::new(DeviceType::ATtiny1626);

    let filename = String::from("D:\\firmware.hex");

    mcu.load_hex(&filename);
    //mcu.load_test_programme();

    let mut cycles = 0;

    while mcu.tick() {
         //Run until break
         cycles += 1;
         if cycles > 100 {
            break;
         }
    }

    mcu.dump_regs();
}
