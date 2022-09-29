use std::rc::Rc;
use std::cell::RefCell;
use std::fs;

use bitvec::prelude::*;

use super::Hardware;
use crate::nets::{PinState, Net, NetState};

#[derive(Debug)]
#[derive(PartialEq)]
enum RxState {
    High, 
    Low,
    Undefined
}

#[derive(Debug)]
#[derive(PartialEq)]
enum UartState {
    Idle,
    Shift,
    Check
}

pub struct SinkUART {
    name: String,
    pin_tx: Rc<RefCell<PinState>>,
    net_rx: Rc<RefCell<Net>>,
    rx_pinstate: RxState,
    rx_state: UartState,
    tx_state: UartState,
    rx_bit: u8,
    tx_bit: u8,
    rx_reg: u16,
    tx_reg: u16,
    rx_time: usize,
    tx_time: usize,
    tics_per_bit: usize,
    out: String, 
    outfile: String
}

impl SinkUART {
    pub fn new(name: String, rx: Rc<RefCell<Net>>, tx: Rc<RefCell<Net>>, filename: String) -> Self {
        let su = SinkUART {
            name, 
            pin_tx: Rc::new(RefCell::new(PinState::DriveH)),
            net_rx: rx,
            rx_pinstate: RxState::Undefined,
            rx_state: UartState::Idle,
            tx_state: UartState::Idle,
            rx_bit: 0,
            tx_bit: 0,
            rx_reg: 0,
            tx_reg: 0,
            rx_time: 0,
            tx_time: 0,
            tics_per_bit: 347,
            out: "".to_string(),
            outfile: filename
        };
        tx.borrow_mut().connect(Rc::downgrade(&su.pin_tx));
        su
    }

    fn out(&mut self, c: u8) {
        self.out.push(c as char);
        //println!("[STDIO] Wrote 0x{:02X} ({})", c as u8, c);
    }

    pub fn out_close(&self) {
        fs::write(&self.outfile, &self.out).expect(&format!("Unable to write uart out to {}.", self.outfile));
    }

    fn tx(&mut self, time: usize, byte: u8) {
        if self.tx_state.eq(&UartState::Idle) {
            let mut b = 1u16;
            b <<= 1;
            b |= byte as u16;
            self.tx_reg = b;
            self.tx_bit = 9; //start + 8N1
            self.tx_state = UartState::Shift;
            self.tx_time = time + self.tics_per_bit;
            *self.pin_tx.borrow_mut() = PinState::DriveL;
        }
    }
}

impl Hardware for SinkUART {
    fn update(&mut self, time: usize) {
        let rx_pinstate_new = match self.net_rx.borrow().state {
            NetState::High => RxState::High,
            NetState::Low => RxState::Low,
            _ => RxState::Undefined
        };

        match self.rx_state {
            UartState::Idle => {
                if rx_pinstate_new.eq(&RxState::Low) & self.rx_pinstate.eq(&RxState::High) {
                    self.rx_state = UartState::Shift;
                    self.rx_bit = 9; // 8N1
                    self.rx_time = time+(self.tics_per_bit*3/2);
                }
            },
            UartState::Shift => {
                if time >= self.rx_time {
                    self.rx_time += self.tics_per_bit;
                    if rx_pinstate_new.eq(&RxState::High) {
                        self.rx_reg |= 1;
                    }
                    self.rx_bit -= 1;
                    if self.rx_bit == 0 {
                        self.rx_state = UartState::Check;
                    } else {
                        self.rx_reg <<= 1;
                    }
                }
            },
            UartState::Check => {
                if self.rx_reg & 1 == 1 {
                    // Stop bit is high, frame OK
                    let mut byte = (self.rx_reg >> 1) as u8;
                    byte.view_bits_mut::<Lsb0>().reverse();
                    self.out(byte);
                }
                self.rx_state = UartState::Idle;
            }
        };

        match self.tx_state {
            UartState::Idle => {},
            UartState::Shift => {
                if time >= self.tx_time {
                    self.tx_time += self.tics_per_bit;
                    if (self.tx_reg & 1) == 1 {
                        *self.pin_tx.borrow_mut() = PinState::DriveH;
                    } else {
                        *self.pin_tx.borrow_mut() = PinState::DriveL;
                    }
                    self.tx_bit -= 1;
                    self.tx_reg >>= 1;
                    if self.tx_bit == 0 {
                        self.tx_state = UartState::Idle;
                    }
                }
            },
            _ => self.tx_state = UartState::Idle
        };

        self.rx_pinstate = rx_pinstate_new;

    }

    fn event(&mut self, time: usize, event: &String) {
        if event.eq_ignore_ascii_case("flush") {
            self.out_close();
        } else {
            let byte = u8::from_str_radix(event.as_str(), 16).unwrap();
            self.tx(time, byte);
        } 
    }
        
}