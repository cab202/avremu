mod devices;
mod cores;
mod memory;

use crate::devices::Device;
use crate::devices::DeviceType;

fn main() {
    let mut mcu = Device::new(DeviceType::ATtiny1626);

    mcu.load_test_programme();

    while mcu.tick() {
         //Run until break
    }

    mcu.dump_regs();
}
