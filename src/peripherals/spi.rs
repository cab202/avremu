use std::alloc::System;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

use crate::peripherals::port::Port;
use crate::memory::MemoryMapped;
use super::Clocked;

use bitvec::prelude::*;

const SPI_CTRLA:    usize = 0x00;
const SPI_CTRLB:    usize = 0x01;
const SPI_INTCTRL:  usize = 0x02;
const SPI_INTFLAGS: usize = 0x03;
const SPI_DATA:     usize = 0x04;

const SPI_PIN_MOSI: usize = 0;
const SPI_PIN_MISO: usize = 1;
const SPI_PIN_SCK:  usize = 2;
const SPI_PIN_SS:   usize = 3;

pub struct Spi {
    name: String,
    regs: [u8; 0x05],
    data_rx: u8,
    data_tx: u8,
    buf_rx: VecDeque<u8>,
    has_data_tx: bool,
    sr_tx: u8,
    sr_rx: u8,
    sr_state: u8,
    port: Rc<RefCell<Port>>,
    pins: [u8; 4],
    port_alt: Rc<RefCell<Port>>,
    pins_alt: [u8; 4],
    pub mux_alt: bool,
    ps_count: u8,
    subinterval: u8, 
    state_sck: bool
}

impl Spi {
    pub fn new(name: String, port: Rc<RefCell<Port>>, pins: [u8; 4], port_alt: Rc<RefCell<Port>>, pins_alt: [u8; 4]) -> Self {
        Spi {
            name,
            regs: [0u8; 0x05],
            data_rx: 0,
            data_tx: 0,
            buf_rx: VecDeque::new(),
            has_data_tx: false,
            sr_tx: 0, 
            sr_rx: 0,
            sr_state: 0,
            port,
            pins, 
            port_alt,
            pins_alt,
            mux_alt: false,
            ps_count: 0,
            subinterval: 0,
            state_sck: false
        }
    }

    fn handle_read_intflags(&self) -> u8 {
        // TODO: Handle flag clear process
        self.regs[SPI_INTFLAGS]
    }
    
    fn handle_write_intflags(&mut self, value: u8) {
         // TODO: Handle hardware, manual clears
    }  

    fn handle_read_data(&mut self) -> u8 {
        // Only master mode implemented
        if self.is_bufen() {
            // Buffered
            if !self.buf_rx.is_empty() {
                self.data_rx = self.buf_rx.pop_front().unwrap();
            }
            if self.buf_rx.is_empty() {
                self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(7, false);
            }
            self.data_rx
        } else {
            // Unbuffered
            self.data_rx
        }
    }

    fn handle_write_data(&mut self, value: u8) {
        // Only master mode implemented
        if self.is_bufen() {
            // Buffered
            if self.sr_state == 0 {
                // Idle
                self.sr_tx = value;
                self.sr_state = 8;
            } else {
                // Transmitting
                self.data_tx = value;
                self.has_data_tx = true;
                self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(5, false);
            }
        } else {
            // Unbuffered
            if self.sr_state > 0 {
                self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(6, true); //WRCOL
            }
            self.sr_tx = value;
            self.sr_state = 8;
        }
    }

    fn is_master(&self) -> bool {
        self.regs[SPI_CTRLA].view_bits::<Lsb0>()[5] 
    }

    fn is_enabled(&self) -> bool {
        self.regs[SPI_CTRLA].view_bits::<Lsb0>()[0] 
    }

    fn is_lsb_first(&self) -> bool {
        self.regs[SPI_CTRLA].view_bits::<Lsb0>()[6] 
    }

    fn prescaler(&self) -> u8 {
        let ps = match (self.regs[SPI_CTRLA] >> 1) & 0x03 {
            0x00 => 4,
            0x01 => 16,
            0x02 => 64,
            0x03 => 128,
            _ => 4
        };

        if self.regs[SPI_CTRLA].view_bits::<Lsb0>()[4] {
            ps >> 1
        } else {
            ps
        }
    }

    fn mode(&self) -> u8 {
        self.regs[SPI_CTRLB] & 0x03
    }

    fn is_ssd(&self) -> bool {
        self.regs[SPI_CTRLB].view_bits::<Lsb0>()[2] 
    }

    fn is_bufen(&self) -> bool {
        self.regs[SPI_CTRLB].view_bits::<Lsb0>()[7] 
    }

    fn is_bufwr(&self) -> bool {
        self.regs[SPI_CTRLB].view_bits::<Lsb0>()[6] 
    } 
    
}

impl MemoryMapped for Spi {
    fn get_size(&self) -> usize {
        self.regs.len() 
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            SPI_CTRLA..=SPI_INTCTRL => (self.regs[address], 0),
            SPI_INTFLAGS => {(self.handle_read_intflags(),0)},
            SPI_DATA => {(self.handle_read_data(),0)},
            _ => panic!("Attempt to access invalid register in SPI peripheral.")
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            SPI_CTRLA..=SPI_INTCTRL => self.regs[address] = value,
            SPI_INTFLAGS => self.handle_write_intflags(value),
            SPI_DATA => {self.handle_write_data(value)},
            _ => panic!("Attempt to access invalid register in SPI peripheral.")
        }
        0
    }
}

impl Clocked for Spi {
    fn tick(&mut self, time: usize) {
        //println!("{} {} {} {}", self.is_enabled(), self.mode(), self.prescaler(), self.is_master());
        // Prescaler
        if self.ps_count == 0 {
            // Reset counter
            self.ps_count = self.prescaler() >> 1; // We need a double speed clock to synth sck

            let mut port;
            let pins;

            if self.mux_alt {
                port = self.port_alt.borrow_mut();
                pins = self.pins_alt;
            } else {
                port = self.port.borrow_mut();
                pins = self.pins;
            }
            
            // TICK!
            if self.is_enabled() {
                if self.is_master() {
                    // Master mode
                    if (self.sr_state == 0) & (self.subinterval == 0) {
                        // Idle
                        match self.mode() {
                            0..=1 => {self.state_sck = false; port.po_out(pins[SPI_PIN_SCK], false)}, // Clock idles low
                            2..=3 => {self.state_sck = true; port.po_out(pins[SPI_PIN_SCK], true)}, // Clock idles high
                            _ => panic!("Invalid SPI mode specified.")
                        }
                    } else {
                        // New transfer
                        if (self.sr_state == 8) & (self.subinterval == 0) {
                            self.subinterval = 16;
                            if !self.is_lsb_first() {
                                self.sr_tx.view_bits_mut::<Lsb0>().reverse();
                            }
                        }

                        match self.mode() {
                            0 => {
                                if self.subinterval < 16 {
                                    self.state_sck = !self.state_sck;
                                    port.po_out(pins[SPI_PIN_SCK], self.state_sck);
                                }
                                if self.subinterval % 2 == 0 {
                                    let mosi = self.sr_tx.view_bits::<Lsb0>()[0];
                                    port.po_out(pins[SPI_PIN_MOSI], mosi);
                                    self.sr_tx >>= 1;
                                    self.sr_rx >>= 1;
                                    self.sr_tx.view_bits_mut::<Lsb0>().set(7, port.get_pinstate(pins[SPI_PIN_MISO]));
                                    self.sr_state -= 1;
                                }
                                if self.subinterval == 1 {
                                    // Reorder recieve data if required
                                    if !self.is_lsb_first() {
                                        self.sr_rx.view_bits_mut::<Lsb0>().reverse();
                                    }
                                    if self.is_bufen() {
                                        // Buffer recieve data
                                        self.buf_rx.push_back(self.sr_rx);
                                        if self.buf_rx.len() > 2 {
                                            // Discard oldest data
                                            self.buf_rx.pop_front();
                                            self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(0, true); // buffer overflow
                                        }
                                        self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(7, true); // recieve complete
                                        // Buffered, check for data in buffer
                                        if self.has_data_tx {
                                            self.sr_tx = self.data_tx;
                                            if !self.is_lsb_first() {
                                                self.sr_tx.view_bits_mut::<Lsb0>().reverse();
                                            }
                                            self.has_data_tx = false;
                                            // Commence new transfer
                                            self.subinterval = 16;
                                            self.sr_state = 8;
                                            self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(5, true); // data reg empty
                                        } else {
                                            self.subinterval = 0;
                                            self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(6, true); // transfer complete
                                        }
                                    } else {
                                        self.data_rx = self.sr_rx;
                                        self.regs[SPI_INTFLAGS].view_bits_mut::<Lsb0>().set(7, true); // transfer complete
                                    }     
                                } else {
                                    self.subinterval -= 1;
                                }
                            },
                            1 => {

                            },
                            2 => {

                            },
                            3 => {

                            },
                            _ => panic!("Invalid SPI mode.")
                        }
                    }


                } else {
                    // Client mode
                    //
                    // Not implemented, but lets at least set the port overrides
                    port.po_dir(pins[SPI_PIN_MOSI], false);
                    port.po_dir(pins[SPI_PIN_SCK], false);
                    port.po_dir(pins[SPI_PIN_SS], false);
                    port.po_out(pins[SPI_PIN_MISO], false);
                }

            } else {
                // We are not enabled, so lets make sure any port overrides are relinquished 
                for i in 0..4 {
                    port.po_out_clear(pins[i]);
                    port.po_dir_clear(pins[i]); 
                }
            }

            
        } else {
            self.ps_count -= 1;
        }
    }
    
}