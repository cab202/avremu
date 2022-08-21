use std::env;

mod devices;
mod cores;
mod memory;
mod peripherals;
mod nets;
mod hardware;
mod boards;
mod events;

use crate::boards::quty::QUTy;
use crate::events::Event;

fn main() {

    let args: Vec<String> = env::args().collect();
    
    let filename_firmware = &args[1];
    let filename_events = &args[2];
    let cycle_limit: u64 = args[3].parse().unwrap();

    let events = Event::from_file(filename_events);

    println!("[FIRMWARE] {}.", filename_firmware);
    println!("[EVENTS] {}: Parsed {} events.", &filename_events, events.len());

    let mut quty = QUTy::new();
    quty.events(events);
    quty.mcu_programme(filename_firmware);
    
    if args.len() > 4 {
        if args[4].eq("debug") {
            quty.core_debug();
        }
    }

    let mut cycles = 0u64;

    println!("[RUN] Cycle limit is {}.", cycle_limit);

    while quty.step() {
         //Run until break
         cycles += 1;
         if cycles == cycle_limit {
            println!("[END] Cycle limit elapsed.");
            break;
         }
    }

    println!("[INFO] Programme terminated after {} cycles.", cycles);

    quty.mcu_dumpstack();
    quty.core_dumpregs();
    quty.mcu_write_stdout();
    
}
