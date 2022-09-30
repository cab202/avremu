use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::memory::MemoryMapped;
use crate::peripherals::InterruptSource;
use crate::peripherals::Clocked;

use super::port::Port;

const USART_RXDATAL:    usize = 0x00;
const USART_RXDATAH:    usize = 0x01;
const USART_TXDATAL:    usize = 0x02;
const USART_TXDATAH:    usize = 0x03;
const USART_STATUS:     usize = 0x04;
const USART_CTRLA:      usize = 0x05;
const USART_CTRLB:      usize = 0x06;
const USART_CTRLC:      usize = 0x07;
const USART_BAUDL:      usize = 0x08;
const USART_BAUDH:      usize = 0x09;
const USART_CTRLD:      usize = 0x0A;
const USART_DBGCTRL:    usize = 0x0B;
const USART_EVCTRL:     usize = 0x0C;
const USART_TXPLCTRL:   usize = 0x0D;
const USART_RXPLCTRL:   usize = 0x0E;

enum USART_MODE {
    NORMAL,
    CLK2X,
    GENAUTO,
    LINAUTO
}

#[derive(Debug)]
#[derive(PartialEq)]
enum RxState {
    High, 
    Low,
    Undefined
}

#[derive(Debug)]
#[derive(PartialEq)]
enum UsartState {
    Idle,
    Shift
}

pub struct Usart {
    name: String, 
    regs: [u8; 0x0F],
    txen: bool,
    rxen: bool,
    port: Rc<RefCell<Port>>,
    port_alt: Rc<RefCell<Port>>,
    pins: [u8; 4],
    pins_alt: [u8; 4],
    pub mux_alt: bool,
    rx_state: UsartState,
    tx_state: UsartState,
    rx_accum: u32,
    tx_accum: u32,
    rx_pinstate: RxState,
    rx_bit: usize,
    tx_bit: usize,
    rx_reg: u16,
    tx_reg: u16,
    rx_buf: VecDeque<u16>,
    tx_buf: VecDeque<u16>
}

impl Usart {
    pub fn new(name: String, port: Rc<RefCell<Port>>, pins: [u8; 4], port_alt: Rc<RefCell<Port>>, pins_alt: [u8; 4]) -> Self {
        let mut usart = Usart {
            name,
            regs: [0; 0x0F],
            txen: false,
            rxen: false,
            port,
            port_alt,
            pins,
            pins_alt,
            mux_alt: false,
            rx_state: UsartState::Idle,
            tx_state: UsartState::Shift,
            rx_accum: 0,
            tx_accum: 0,
            rx_pinstate: RxState::Undefined,
            rx_bit: 0,
            tx_bit: 0,
            rx_reg: 0,
            tx_reg: 0,
            rx_buf: VecDeque::new(),
            tx_buf: VecDeque::new()
        };
        usart.regs[USART_STATUS] = 0x20;
        usart
    }

    fn rxen(&self) -> bool {
        (self.regs[USART_CTRLB] & 0x80) != 0
    }

    fn txen(&self) -> bool {
        (self.regs[USART_CTRLB] & 0x40) != 0
    }

    fn mode(&self) -> USART_MODE {
        match (self.regs[USART_CTRLB] >> 1) & 0x03 {
            0 => USART_MODE::NORMAL,
            1 => USART_MODE::CLK2X,
            2 => USART_MODE::GENAUTO,
            3 => USART_MODE::LINAUTO,
            _ => panic!("ERROR! Invalid USART mode.")
        }
    }

    fn baud_inc(&self) -> u32 {
        let mut baud = ((self.regs[USART_BAUDH] as u64) << 8) | (self.regs[USART_BAUDL] as u64);
        match self.mode() {
            USART_MODE::NORMAL => baud *= 16,
            USART_MODE::CLK2X => baud *= 8,
            _ => {}
        }
        let mut inc = (0x100000000u64 * 64) / baud.max(64); // note tick = clk_per so cancels
        inc as u32
    }

    fn dre(&self) -> bool {
        (self.regs[USART_STATUS] & 0x20) != 0
    }
}

impl MemoryMapped for Usart {
    fn get_size(&self) -> usize {
        self.regs.len()
    }

    fn read(&mut self, address: usize) -> (u8, usize) {
        match address {
            USART_RXDATAL => {
                //This logic only works for < 9 bit frames
                if self.rx_buf.len() > 0 {
                    let data = self.rx_buf.pop_front().unwrap();
                    self.regs[USART_RXDATAL] = data as u8;
                    self.regs[USART_RXDATAH] &= 0xC0;
                    self.regs[USART_RXDATAH] |= (data >> 8) as u8;
                    if self.rx_buf.len() == 0 {
                        self.regs[USART_RXDATAH] &= 0x7F; // Clear RXCIF (buffer empty)
                        self.regs[USART_STATUS] &= 0x7F; // Clear RXCIF (buffer empty)
                    }
                }
                (self.regs[USART_RXDATAL], 0)
            },
            USART_RXDATAH => {
                //This logic only works for < 9 bit frames
                if self.rx_buf.len() > 0 {
                    let data = self.rx_buf.front().unwrap();
                    self.regs[USART_RXDATAH] &= 0xC0;
                    self.regs[USART_RXDATAH] |= (data >> 8) as u8;
                }
                (self.regs[USART_RXDATAH], 0)
            },
            USART_RXDATAL..=USART_RXPLCTRL => (self.regs[address], 0),
            _ => (0, 0)
        }
    }

    fn write(&mut self, address: usize, value: u8) -> usize {
        match address {
            //RXDATA is read only
            USART_TXDATAL => {
                println!("[USART] TXDATAH: 0x{:02X}", value);
                if self.dre() {
                    if self.tx_state.eq(&UsartState::Idle) {
                        // start bit
                        if self.mux_alt {
                            self.port_alt.borrow_mut().po_out(self.pins_alt[1], false);
                        } else {
                            self.port.borrow_mut().po_out(self.pins[1], false);
                        }
                        self.tx_bit = 9; //check??
                        self.tx_reg = (value as u16) | 0x0100; // Add stop bit
                        self.tx_accum = 0;
                        self.tx_state = UsartState::Shift;
                    } else {
                        self.tx_buf.push_back(value as u16);
                        self.regs[USART_TXDATAL] = value;
                        if self.tx_buf.len() > 1 {
                            self.regs[USART_STATUS] &= !0x20; // Clear DREIF
                        }
                    }
                }
            },
            USART_TXDATAH => {
                println!("[WARNING] 9-bit mode is not implemented for USART in this emulator. These bits will be ignored.");
            },
            USART_STATUS => {
                self.regs[USART_STATUS] &= !value | 0xA5; // Flags cleared by writing 1
                if (value & 1) == 1 {
                    self.regs[USART_STATUS] |= 1;
                } else {
                    self.regs[USART_STATUS] &= 0xFE;
                }
            },
            USART_CTRLA => {
                self.regs[USART_CTRLA] = value;
                if (value & 0x0F) != 0 {
                    println!("[WARNING] LBME, ABEIE and RS485 features are not implemented for USART in this emulator. These bits will be ignored.");
                }
            },
            USART_CTRLB => {
                self.regs[USART_CTRLB] = value;
                if self.txen() {
                    if self.mux_alt {
                        self.port_alt.borrow_mut().po_out(self.pins_alt[1], true);
                    } else {
                        self.port.borrow_mut().po_out(self.pins[1], true);
                    }
                } else {
                    if self.mux_alt {
                        self.port_alt.borrow_mut().po_out_clear(self.pins_alt[1]);
                    } else {
                        self.port.borrow_mut().po_out_clear(self.pins[1]);
                    }
                }
                if (value & 0x1D) != 0 {
                    println!("[WARNING] SFDEN, ODME, GENAUTO, LINAUTO and MPCM features are not implemented for USART in this emulator. These bits will be ignored.");
                }
            },
            USART_CTRLC => {
                self.regs[USART_CTRLC] = value;
                if value != 0x03 {
                    println!("[WARNING] Only asynchronous 8N1 mode is implemented for USART in this emulator. These bits will be ignored.");
                }
            },
            USART_BAUDL..=USART_BAUDH => {
                self.regs[address] = value;
            },
            USART_CTRLD => {
                println!("[WARNING] CTRLD features are not implemented for USART in this emulator. This register will be ignored.");
                self.regs[USART_CTRLD] = value;
            },
            USART_DBGCTRL => {
                println!("[WARNING] DBGCTRL features are not implemented for USART in this emulator. This register will be ignored.");
                self.regs[USART_DBGCTRL] = value;
            },
            USART_EVCTRL => {
                println!("[WARNING] EVCTRL features are not implemented for USART in this emulator. This register will be ignored.");
                self.regs[USART_EVCTRL] = value;
            },
            USART_TXPLCTRL => {
                println!("[WARNING] TXPLCTRL features are not implemented for USART in this emulator. This register will be ignored.");
                self.regs[USART_TXPLCTRL] = value;
            },
            USART_RXPLCTRL => {
                println!("[WARNING] RXPLCTRL features are not implemented for USART in this emulator. This register will be ignored.");
                self.regs[USART_RXPLCTRL] = value;
            },
            _ => {}
        }
        0
    }
}

impl InterruptSource for Usart {
    fn interrupt(&self, mask: u8) -> bool {
        (self.regs[USART_STATUS] & self.regs[USART_CTRLA] & mask) != 0x00
    }
}

impl Clocked for Usart {
    fn tick(&mut self, time: usize) {
        // new Rx pinstate
        let rx_port_pinstate = if self.mux_alt {
            self.port_alt.borrow().get_pinstate(self.pins_alt[0])
        } else {
            self.port.borrow().get_pinstate(self.pins[0])
        };
        let rx_pinstate_new = if rx_port_pinstate {
            RxState::High
        } else {
            RxState::Low
        };

        // increment accumulators
        let (mut rx_accum_new, _) = self.rx_accum.overflowing_add(self.baud_inc());
        let (mut tx_accum_new, _) = self.tx_accum.overflowing_add(self.baud_inc());
        
        if self.rxen() {
            match self.rx_state {
                UsartState::Idle => {
                    if rx_pinstate_new.eq(&RxState::Low) & self.rx_pinstate.eq(&RxState::High) {
                        self.rx_state = UsartState::Shift;
                        self.rx_bit = 10; // 8N1
                        rx_accum_new = 0x80000000; // half bit
                        if self.rx_buf.len() > 1 {
                            self.regs[USART_RXDATAH] |= 0x40; // Buffer overflow
                        } 
                    }
                },
                UsartState::Shift => {
                    if rx_accum_new < self.rx_accum {
                        if rx_pinstate_new.eq(&RxState::High) {
                            self.rx_reg |= 1;
                        }
                        self.rx_bit -= 1;
                        if self.rx_bit == 0 {
                            self.rx_state = UsartState::Idle;
                            self.rx_reg = self.rx_reg.reverse_bits(); // Stop bit now MSB
                            let ferr = if (self.rx_reg & 0x80) == 0x80 {1 << 10} else {0u16};
                            let perr = 0u16;
                            self.rx_reg >>= 7;  // D0 is now LSB
                            self.rx_reg &= 0x00FF;  // Adjust this if 9 bits required
                            self.rx_buf.push_back(ferr | perr | self.rx_reg);
                            self.regs[USART_RXDATAH] |= 0x80; // Set RXCIF (unread data)
                            self.regs[USART_STATUS] |= 0x80; // Set RXCIF (unread data)
                            if self.rx_buf.len() > 2 {
                                self.rx_buf.pop_front(); // Overflow, discard oldest data
                            }   
                        } else {
                            self.rx_reg <<= 1;
                        }
                    }
                }
            }
        } else {
            self.rx_state = UsartState::Idle;
        }

        if self.txen() {
            match self.tx_state {
                UsartState::Idle => {},
                UsartState::Shift => {
                    if tx_accum_new < self.tx_accum {
                        if self.mux_alt {
                            self.port_alt.borrow_mut().po_out(self.pins_alt[1], (self.tx_reg & 1) == 1);
                        } else {
                            self.port.borrow_mut().po_out(self.pins[1], (self.tx_reg & 1) == 1);
                        }
                        println!("[@{:08X}] USART Tx, Bit {}: {:}", time, self.tx_bit, self.tx_reg & 1);
                        self.tx_reg >>= 1;
                        self.tx_bit -= 1;
                        if self.tx_bit == 0 {
                            if self.tx_buf.len() > 0 {
                                self.tx_reg = (self.tx_buf.pop_front().unwrap() << 1) | 0x0200; // add start and stop bits
                                self.tx_bit = 10; // check???
                                if !self.dre() {
                                    self.regs[USART_STATUS] |= 0x20; // Set DREIF
                                }
                            } else {
                                self.tx_state = UsartState::Idle;
                                self.tx_accum = 0;
                                self.regs[USART_STATUS] |= 0x40;  // Set TXCIF (buffer empty)
                            }                            
                        }
                    }
                }
            }
        } else {
            self.tx_state = UsartState::Idle;
        }

        // update internal state
        self.rx_accum = rx_accum_new;
        self.tx_accum = tx_accum_new;
        self.rx_pinstate = rx_pinstate_new;
    }
}