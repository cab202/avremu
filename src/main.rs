use std::env;
use std::rc::Rc;
use std::cell::RefCell;

mod devices;
mod cores;
mod memory;
mod peripherals;
mod nets;
mod hardware;
mod boards;

use crate::devices::Device;
use crate::devices::DeviceType;
use crate::nets::{Net, PinState};
use crate::hardware::led::Led;
use crate::boards::quty::QUTy;


fn main() {

    //let pin = Rc::new(RefCell::new(PinState::Open));
    //let net = Rc::new(RefCell::new(Net::new()));
    //let mut led = Led::new("TEST".to_string(), true, Rc::clone(&net));
    //net.borrow_mut().connect(Rc::downgrade(&pin));
    //
    //net.borrow_mut().update();
    //led.update();
//
    //*pin.borrow_mut() = PinState::DriveH;
    //net.borrow_mut().update();
    //led.update();
//
    //*pin.borrow_mut() = PinState::DriveL;
    //net.borrow_mut().update();
    //led.update();
//
    //*pin.borrow_mut() = PinState::WeakPullDown;
    //net.borrow_mut().update();
    //led.update();
//
    //*pin.borrow_mut() = PinState::Open;
    //net.borrow_mut().update();
    //led.update();
//
    //*pin.borrow_mut() = PinState::WeakPullUp;
    //net.borrow_mut().update();
    //led.update();

    let args: Vec<String> = env::args().collect();
    
    let filename = &args[1];
    let cycle_limit: u64 = args[2].parse().unwrap();

    println!("[FIRMWARE] {}.", filename);

    let mut quty = QUTy::new();
    quty.mcu_programme(filename);
    
    if args.len() > 3 {
        if args[3].eq("debug") {
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
    
}
