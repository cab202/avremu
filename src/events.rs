use std::fs;

use regex::Regex;

pub type Events = Vec<Event>;

pub struct Event {
    pub time: u64,
    pub device: String,
    pub event: String
}

impl Event {
    pub fn from_file(filename: &String) -> Events {
        let file_events = fs::read_to_string(filename);

        let mut events = Vec::new();

        if file_events.is_err() {
            println!("[EVENTS] Couldn't open {}. {}", filename, file_events.unwrap_err());
            return events;
        }

        let event_str = file_events.unwrap(); 
        let re_events = Regex::new("@([0-9A-F-a-f]+)\\s+(.+):\\s+(.+)\\n+").unwrap();
        let caps_events = re_events.captures_iter(&event_str);
    
        for cap in caps_events {
            
            events.push(
                Event {
                    time: u64::from_str_radix(&cap[1], 16).unwrap(), 
                    device: cap[2].trim().to_owned(),
                    event: cap[3].trim().to_owned(),
                }
            );
        }

        //for e in &events {
        //    println!("{} {}: {} ({} {})", e.time, e.device, e.event, e.device.len(), e.event.len());
        //}

        events
    }
}