use std::fs::File;
use std::path::Path;

use clap::Parser;

mod boards;
mod cores;
mod devices;
mod events;
mod hardware;
mod memory;
mod nets;
mod peripherals;

use crate::boards::quty::QUTy;
use crate::events::Event;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref CLI: Cli = Cli::parse();
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
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

    /// Output all net state transitions
    #[arg(short = 'n', long)]
    net_all: bool,

    /// Output net transitions from undefined state only
    #[arg(short = 'u', long)]
    net_undef: bool,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let firmware = &CLI.firmware;
    {
        let file = File::open(Path::new(firmware));
        match file {
            Ok(..) => println!("[FIRMWARE] {}.", firmware),
            Err(e) => {
                println!("[FIRMWARE] Couldn't open {}. {}", firmware, e);
                return;
            }
        }
        // Drop file
    }

    let events = match &CLI.events {
        Some(filename) => {
            let events = Event::from_file(filename);
            println!("[EVENTS] {}: Parsed {} events.", &filename, events.len());
            events
        }
        None => Vec::new(),
    };

    let time_limit = match CLI.timeout {
        Some(timeout) => {
            println!("[RUN] Time limit is {} ns.", timeout);
            timeout
        }
        None => {
            println!("[RUN] No emulation time limit specified.");
            u64::MAX
        }
    };

    let mut quty = QUTy::new();
    quty.events(events);
    quty.mcu_programme(firmware);

    if CLI.debug {
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

    if CLI.dump_stack {
        quty.mcu_dumpstack();
    }

    if CLI.dump_regs {
        quty.core_dumpregs();
    }

    if CLI.dump_stdout {
        quty.mcu_write_stdout();
    }
}
