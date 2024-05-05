use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::devices::Device;
use crate::devices::DeviceType;

use crate::nets::Net;
use crate::nets::NetState;
use crate::nets::PinState;

use crate::hardware::display::Display;
use crate::hardware::ic74hc595::IC74HC595;
use crate::hardware::led::Led;
use crate::hardware::pot::Pot;
use crate::hardware::pushbutton::Pushbutton;
use crate::hardware::sinkpwm::SinkPwm;
use crate::hardware::sinkuart::SinkUART;
use crate::hardware::Hardware;

use crate::events::Events;

pub struct QUTy {
    hw: HashMap<String, Box<dyn Hardware>>,
    nets: HashMap<String, Rc<RefCell<Net>>>,
    mcu: Device,
    time: u64,
    events: Events,
}

impl QUTy {
    pub fn new() -> Self {
        let mcu = Device::new(DeviceType::ATtiny1626);

        let net_gnd = Rc::new(RefCell::new(Net::new("GND".to_string())));
        let net_vdd = Rc::new(RefCell::new(Net::new("VDD".to_string())));

        net_gnd.borrow_mut().state = NetState::Low;
        net_vdd.borrow_mut().state = NetState::High;

        let nets = HashMap::from([
            (
                "PA1_DISP_LATCH".to_string(),
                Rc::new(RefCell::new(Net::new("PA1_DISP_LATCH".to_string()))),
            ),
            (
                "PA2_POT".to_string(),
                Rc::new(RefCell::new(Net::new("PA2_POT".to_string()))),
            ),
            (
                "PA3_CLK".to_string(),
                Rc::new(RefCell::new(Net::new("PA3_CLK".to_string()))),
            ),
            (
                "PA4_BUTTON0".to_string(),
                Rc::new(RefCell::new(Net::new("PA4_BUTTON0".to_string()))),
            ),
            (
                "PA5_BUTTON1".to_string(),
                Rc::new(RefCell::new(Net::new("PA5_BUTTON1".to_string()))),
            ),
            (
                "PA6_BUTTON2".to_string(),
                Rc::new(RefCell::new(Net::new("PA6_BUTTON2".to_string()))),
            ),
            (
                "PA7_BUTTON3".to_string(),
                Rc::new(RefCell::new(Net::new("PA7_BUTTON3".to_string()))),
            ),
            (
                "PB0_BUZZER".to_string(),
                Rc::new(RefCell::new(Net::new("PB0_BUZZER".to_string()))),
            ),
            (
                "PB1_DISP_EN".to_string(),
                Rc::new(RefCell::new(Net::new("PB1_DISP_EN".to_string()))),
            ),
            (
                "PB2_UART_TX".to_string(),
                Rc::new(RefCell::new(Net::new("PB2_UART_TX".to_string()))),
            ),
            (
                "PB3_UART_RX".to_string(),
                Rc::new(RefCell::new(Net::new("PB3_UART_RX".to_string()))),
            ),
            (
                "PB4_UART_RX".to_string(),
                Rc::new(RefCell::new(Net::new("PB4_UART_RX".to_string()))),
            ),
            (
                "PB5_DISP_DP".to_string(),
                Rc::new(RefCell::new(Net::new("PB5_DISP_DP".to_string()))),
            ),
            (
                "PC0_SPI_CLK".to_string(),
                Rc::new(RefCell::new(Net::new("PC0_SPI_CLK".to_string()))),
            ),
            (
                "PC1_SPI_MISO".to_string(),
                Rc::new(RefCell::new(Net::new("PC1_SPI_MISO".to_string()))),
            ),
            (
                "PC2_SPI_MOSI".to_string(),
                Rc::new(RefCell::new(Net::new("PC2_SPI_MOSI".to_string()))),
            ),
            (
                "PC3_SPI_CS".to_string(),
                Rc::new(RefCell::new(Net::new("PC3_SPI_CS".to_string()))),
            ),
            (
                "U2_Q0".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q0".to_string()))),
            ),
            (
                "U2_Q1".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q1".to_string()))),
            ),
            (
                "U2_Q2".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q2".to_string()))),
            ),
            (
                "U2_Q3".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q3".to_string()))),
            ),
            (
                "U2_Q4".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q4".to_string()))),
            ),
            (
                "U2_Q5".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q5".to_string()))),
            ),
            (
                "U2_Q6".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q6".to_string()))),
            ),
            (
                "U2_Q7".to_string(),
                Rc::new(RefCell::new(Net::new("U2_Q7".to_string()))),
            ),
        ]);

        mcu.ports[0]
            .borrow_mut()
            .connect(1, Rc::clone(nets.get("PA1_DISP_LATCH").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(2, Rc::clone(nets.get("PA2_POT").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(3, Rc::clone(nets.get("PA3_CLK").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(4, Rc::clone(nets.get("PA4_BUTTON0").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(5, Rc::clone(nets.get("PA5_BUTTON1").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(6, Rc::clone(nets.get("PA6_BUTTON2").unwrap()));
        mcu.ports[0]
            .borrow_mut()
            .connect(7, Rc::clone(nets.get("PA7_BUTTON3").unwrap()));

        mcu.ports[1]
            .borrow_mut()
            .connect(0, Rc::clone(nets.get("PB0_BUZZER").unwrap()));
        mcu.ports[1]
            .borrow_mut()
            .connect(1, Rc::clone(nets.get("PB1_DISP_EN").unwrap()));
        mcu.ports[1]
            .borrow_mut()
            .connect(2, Rc::clone(nets.get("PB2_UART_TX").unwrap()));
        mcu.ports[1]
            .borrow_mut()
            .connect(3, Rc::clone(nets.get("PB3_UART_RX").unwrap()));
        mcu.ports[1]
            .borrow_mut()
            .connect(4, Rc::clone(nets.get("PB4_UART_RX").unwrap()));
        mcu.ports[1]
            .borrow_mut()
            .connect(5, Rc::clone(nets.get("PB5_DISP_DP").unwrap()));

        mcu.ports[2]
            .borrow_mut()
            .connect(0, Rc::clone(nets.get("PC0_SPI_CLK").unwrap()));
        mcu.ports[2]
            .borrow_mut()
            .connect(1, Rc::clone(nets.get("PC1_SPI_MISO").unwrap()));
        mcu.ports[2]
            .borrow_mut()
            .connect(2, Rc::clone(nets.get("PC2_SPI_MOSI").unwrap()));
        mcu.ports[2]
            .borrow_mut()
            .connect(3, Rc::clone(nets.get("PC3_SPI_CS").unwrap()));

        let mut sr = IC74HC595::new("U2".to_string());
        for i in 0..8 {
            sr.connect_q(i, Rc::clone(nets.get(&format!("U2_Q{}", i)).unwrap()));
        }
        sr.connect("ds", Rc::clone(nets.get("PC2_SPI_MOSI").unwrap()));
        sr.connect("shcp", Rc::clone(nets.get("PC0_SPI_CLK").unwrap()));
        sr.connect("stcp", Rc::clone(nets.get("PA1_DISP_LATCH").unwrap()));
        sr.connect("oe_n", Rc::clone(&net_gnd));
        sr.connect("mr_n", Rc::clone(&net_vdd));

        let mut disp = Display::new("DS1".to_string());
        for i in 0..7 {
            disp.connect_seg(i, Rc::clone(nets.get(&format!("U2_Q{}", i)).unwrap()));
        }
        disp.connect("en", Rc::clone(nets.get("PB1_DISP_EN").unwrap()));
        disp.connect("digit", Rc::clone(nets.get("U2_Q7").unwrap()));

        let mut hw: HashMap<String, Box<dyn Hardware>> = HashMap::new();
        hw.insert(
            "DS1-DP".to_string(),
            Box::new(Led::new(
                "DS1-DP".to_string(),
                false,
                Rc::clone(nets.get("PB5_DISP_DP").unwrap()),
            )),
        );
        hw.insert(
            "S1".to_string(),
            Box::new(Pushbutton::new(
                "S1".to_string(),
                false,
                Rc::clone(nets.get("PA4_BUTTON0").unwrap()),
            )),
        );
        hw.insert(
            "S2".to_string(),
            Box::new(Pushbutton::new(
                "S2".to_string(),
                false,
                Rc::clone(nets.get("PA5_BUTTON1").unwrap()),
            )),
        );
        hw.insert(
            "S3".to_string(),
            Box::new(Pushbutton::new(
                "S3".to_string(),
                false,
                Rc::clone(nets.get("PA6_BUTTON2").unwrap()),
            )),
        );
        hw.insert(
            "S4".to_string(),
            Box::new(Pushbutton::new(
                "S4".to_string(),
                false,
                Rc::clone(nets.get("PA7_BUTTON3").unwrap()),
            )),
        );
        hw.insert(
            "P1".to_string(),
            Box::new(SinkPwm::new(
                "P1".to_string(),
                "BUZZER".to_string(),
                Rc::clone(nets.get("PB0_BUZZER").unwrap()),
                PinState::Open,
            )),
        );
        hw.insert("U2".to_string(), Box::new(sr));
        hw.insert("DS1".to_string(), Box::new(disp));
        hw.insert(
            "R9".to_string(),
            Box::new(SinkPwm::new(
                "DISP_EN".to_string(),
                "PWM".to_string(),
                Rc::clone(nets.get("PB1_DISP_EN").unwrap()),
                PinState::WeakPullUp,
            )),
        );
        hw.insert(
            "R1".to_string(),
            Box::new(Pot::new(
                "R1".to_string(),
                Rc::clone(nets.get("PA2_POT").unwrap()),
                0.5,
            )),
        );
        hw.insert(
            "U5".to_string(),
            Box::new(SinkUART::new(
                "U5".to_string(),
                Rc::clone(nets.get("PB2_UART_TX").unwrap()),
                Rc::clone(nets.get("PB3_UART_RX").unwrap()),
                "./uart.txt".to_string(),
            )),
        );

        let mut quty = QUTy {
            hw,
            nets,
            mcu,
            time: 0,
            events: Vec::new(),
        };

        for net in &quty.nets {
            net.1.borrow_mut().update(0);
        }
        for dev in &mut quty.hw {
            dev.1.update(0);
        }
        quty.mcu.update(0);

        quty
    }

    pub fn step(&mut self) -> u64 {
        //let timestep = 300; // Default 300 ns => 3.3 MHz

        if !self.events.is_empty() {
            while self.time >= self.events[0].time {
                self.hw
                    .get_mut(&self.events[0].device)
                    .unwrap()
                    .event(self.time, &self.events[0].event);
                self.events.remove(0);
                if self.events.is_empty() {
                    break;
                }
            }
        }

        let timestep = self.mcu.tick(self.time);
        for net in &self.nets {
            net.1.borrow_mut().update(self.time);
        }
        for hw in &mut self.hw {
            hw.1.update(self.time);
        }
        self.mcu.update(self.time);

        // Update the time at the end (once we know what the micro step was)
        self.time += timestep;

        // Force UART flush if program terminates before event is called
        if timestep == 0
            && self
                .events
                .iter()
                .any(|e| e.device == "U5" && e.event == "flush")
        {
            self.hw.get_mut("U5").unwrap().event(self.time, "flush");
        }

        timestep
    }

    pub fn core_debug(&mut self) {
        self.mcu.core.debug(true);
    }

    pub fn core_dumpregs(&self) {
        self.mcu.dump_regs();
    }

    pub fn mcu_dumpstack(&self) {
        self.mcu.dump_stack();
    }

    pub fn mcu_programme(&mut self, filename: &str) {
        self.mcu.load_hex(filename);
    }

    pub fn mcu_write_stdout(&self) {
        self.mcu.stdio.borrow().out_close();
    }

    pub fn events(&mut self, events: Events) {
        self.events = events;
    }
}
