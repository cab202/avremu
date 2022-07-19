use std::env;

mod devices;
mod cores;
mod memory;
mod peripherals;
mod nets;

use crate::devices::Device;
use crate::devices::DeviceType;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let filename = &args[1];
    let cycle_limit: u64 = args[2].parse().unwrap();

    let mut mcu = Device::new(DeviceType::ATtiny1626);

    println!("[FIRMWARE] {}.", filename);

    mcu.load_hex(&filename);
    //mcu.core.debug(true);

    let mut cycles = 0u64;

    println!("[RUN] Cycle limit is {}.", cycle_limit);

    while mcu.tick() {
         //Run until break
         cycles += 1;
         if cycles == cycle_limit {
            println!("[END] Cycle limit elapsed.");
            break;
         }
    }

    println!("[INFO] Programme terminated after {} cycles.", cycles);

    mcu.dump_stack();
    mcu.dump_regs();
    
}
