use std::cell::RefCell;
use std::rc::Rc;

use bitvec::prelude::*;

use super::Hardware;
use crate::nets::{Net, NetState, PinState};

#[derive(Debug, PartialEq)]
enum ClockState {
    High,
    Low,
    Undefined,
}

#[allow(dead_code)]
pub struct IC74HC595 {
    name: String,
    pins_out: [Rc<RefCell<PinState>>; 8],
    pin_q7s: Rc<RefCell<PinState>>,
    //pin_stcp: Rc<RefCell<PinState>>,
    //pin_ds: Rc<RefCell<PinState>>,
    //pin_shcp: Rc<RefCell<PinState>>,
    //pin_oe_n: Rc<RefCell<PinState>>,
    //pin_mr_n: Rc<RefCell<PinState>>,
    nets_out: [Rc<RefCell<Net>>; 8],
    net_stcp: Rc<RefCell<Net>>,
    net_ds: Rc<RefCell<Net>>,
    net_shcp: Rc<RefCell<Net>>,
    net_oe_n: Rc<RefCell<Net>>,
    net_mr_n: Rc<RefCell<Net>>,
    reg_shift: u8,
    reg_latch: u8,
    state_shcp: ClockState,
    state_stcp: ClockState,
}

impl IC74HC595 {
    pub fn new(name: String) -> Self {
        IC74HC595 {
            name,
            pins_out: [
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
                Rc::new(RefCell::new(PinState::Open)),
            ],
            pin_q7s: Rc::new(RefCell::new(PinState::Open)),
            //pin_stcp: Rc::new(RefCell::new(PinState::Open)),
            //pin_shcp: Rc::new(RefCell::new(PinState::Open)),
            //pin_ds: Rc::new(RefCell::new(PinState::Open)),
            //pin_oe_n: Rc::new(RefCell::new(PinState::Open)),
            //pin_mr_n: Rc::new(RefCell::new(PinState::Open)),
            nets_out: [
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
                Rc::new(RefCell::new(Net::new("".to_string()))),
            ],
            net_stcp: Rc::new(RefCell::new(Net::new("".to_string()))),
            net_shcp: Rc::new(RefCell::new(Net::new("".to_string()))),
            net_ds: Rc::new(RefCell::new(Net::new("".to_string()))),
            net_oe_n: Rc::new(RefCell::new(Net::new("".to_string()))),
            net_mr_n: Rc::new(RefCell::new(Net::new("".to_string()))),
            reg_shift: 0,
            reg_latch: 0,
            state_shcp: ClockState::Undefined,
            state_stcp: ClockState::Undefined,
        }
    }

    pub fn connect_q(&mut self, n: usize, net: Rc<RefCell<Net>>) {
        self.nets_out[n] = net;
        self.nets_out[n]
            .borrow_mut()
            .connect(Rc::downgrade(&self.pins_out[n]));
    }

    pub fn connect(&mut self, pin_name: &str, net: Rc<RefCell<Net>>) {
        match pin_name {
            "shcp" => self.net_shcp = net,
            "stcp" => self.net_stcp = net,
            "ds" => self.net_ds = net,
            "mr_n" => self.net_mr_n = net,
            "oe_n" => self.net_oe_n = net,
            _ => {}
        }
    }
}

impl Hardware for IC74HC595 {
    fn update(&mut self, _time: u64) {
        let state_shcp_new = match self.net_shcp.borrow().state {
            NetState::High => ClockState::High,
            NetState::Low => ClockState::Low,
            _ => ClockState::Undefined,
        };
        //println!("SR: SHCP is {:?}", state_shcp_new);
        let state_stcp_new = match self.net_stcp.borrow().state {
            NetState::High => ClockState::High,
            NetState::Low => ClockState::Low,
            _ => ClockState::Undefined,
        };
        //println!("SR: STCP is {:?}", state_stcp_new);
        if self.net_mr_n.borrow().state.eq(&NetState::Low) {
            //println!("SR: Mater reset");
            *self.pin_q7s.borrow_mut() = PinState::DriveL;
            self.reg_shift = 0;
            if state_stcp_new.eq(&ClockState::High) & self.state_stcp.eq(&ClockState::Low) {
                //Rising edge store clock
                //println!("SR: Rising edge store clock");
                self.reg_latch = 0;
            }
        } else {
            // On simultaneous clock rising edges store takes pre-shift values
            if state_stcp_new.eq(&ClockState::High) & self.state_stcp.eq(&ClockState::Low) {
                //Rising edge store clock
                //println!("SR: Rising edge store clock");
                self.reg_latch = self.reg_shift;
            }
            if state_shcp_new.eq(&ClockState::High) & self.state_shcp.eq(&ClockState::Low) {
                //Rising edge shift clock
                //println!("SR: Rising edge shift clock");
                *self.pin_q7s.borrow_mut() = if self.reg_shift.view_bits::<Lsb0>()[6] {
                    PinState::DriveH
                } else {
                    PinState::DriveL
                };
                self.reg_shift <<= 1;
                self.reg_shift |= if self.net_ds.borrow_mut().state.eq(&NetState::High) {
                    1
                } else {
                    0
                };
            }
        }

        if self.net_oe_n.borrow().state.eq(&NetState::Low) {
            //println!("SR: Output enabled");
            for i in 0..8 {
                *self.pins_out[i].borrow_mut() = if self.reg_latch.view_bits::<Lsb0>()[i] {
                    PinState::DriveH
                } else {
                    PinState::DriveL
                }
            }
        } else {
            for i in 0..8 {
                *self.pins_out[i].borrow_mut() = PinState::Open;
            }
        }

        self.state_shcp = state_shcp_new;
        self.state_stcp = state_stcp_new;
    }
}
