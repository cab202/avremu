use std::fs::File;
use std::path::Path;

use clap::Parser;

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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Microcontroller firmware to load in .HEX format
    firmware: String,

    /// Specify event file for hardware events
    #[arg(short, long)]
    events: Option<String>,

    /// Specify emulation runtime limit in nanoseconds
    #[arg(short, long)]
    timeout: Option<u64>,

    /// Dump stack to stdout on termination
    #[arg(short = 's', long)]
    dump_stack: bool,

    /// Dump working register values to stdout on termination
    #[arg(short = 'r', long)]
    dump_regs: bool,

    /// Dump output of stdio pseudo-peripheral to file stdout.txt on termination
    #[arg(short = 'o', long)]
    dump_stdout: bool,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool
}

fn main() {
    let cli = Cli::parse();

    //let args: Vec<String> = env::args().collect();
    
    let firmware = &cli.firmware;
    {
        let file = File::open(Path::new(firmware)); 
        match file {
            Ok(..) => println!("[FIRMWARE] {}.", firmware),
            Err(e) => {
                println!("[FIRMWARE] Couldn't open {}. {}", firmware, e);
                return
            }
        }
        // Drop file
    }

    let events = match cli.events {
        Some(filename) => {
            let events = Event::from_file(&filename);
            println!("[EVENTS] {}: Parsed {} events.", &filename, events.len());
            events
        },
        None => Vec::new()
    };

    let time_limit = match cli.timeout {
        Some(timeout) => {
            println!("[RUN] Time limit is {} ns.", timeout);
            timeout
        },
        None => {
            println!("[RUN] No emulation time limit specified.");
            u64::MAX
        }
    };

    let mut quty = QUTy::new();
    quty.events(events);
    quty.mcu_programme(firmware);

    if cli.debug {
        quty.core_debug();
    }

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

    if cli.dump_stack {
        quty.mcu_dumpstack();
    }

    if cli.dump_regs {
        quty.core_dumpregs();
    }

    if cli.dump_stdout {
        quty.mcu_write_stdout();
    }
    
}
