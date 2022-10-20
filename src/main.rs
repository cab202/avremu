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
    let time_limit = u64::from_str_radix(&args[3], 16).unwrap();

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

    println!("[RUN] Time limit is {} ns.", time_limit);

    let mut time = 0u64;
    let mut time_step;
    loop {
        time_step = quty.step();

        // Board returns a step time of 0 to indicate termination
        if time_step == 0 {
            break;
        }

        time += time_step;

        // Check time limit
        if time >= time_limit {
            println!("[END] Time limit elapsed.");
            break;
         }
    }

    println!("[INFO] Programme terminated after {} ns.", time);

    quty.mcu_dumpstack();
    quty.core_dumpregs();
    quty.mcu_write_stdout();
    
}
