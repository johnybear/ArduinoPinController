extern crate chrono;
extern crate serialport;
extern crate serde_json;
use chrono::prelude::*;
use serde_json::{Value, Error};
use chrono::Duration;
use std::sync::mpsc::{Sender, Receiver};


pub struct Command {
    pin: u8,
    time_of_creation: DateTime<Local>,
    timer: i64,
    status: u8
}

impl Command {
    
    pub fn new(val: Value) -> Command {
        Command{
            pin: val["pin"].as_u64().unwrap_or(0) as u8,
            time_of_creation: Local::now(),
            timer: val["timer"].as_i64().unwrap_or(0),
            status: val["status"].as_u64().unwrap_or(0) as u8
        }
    }

    pub fn ready_to_execute(&self, sender: &Sender<(u8, u8)>) -> bool{
        let current_time: DateTime<Local> = Local::now();
        let timer = self.time_of_creation + Duration::seconds(self.timer);
        if timer <= current_time {
            match sender.send((self.pin, self.status)){
                Ok(b) => (),
                Err(b) => println!("Error {}, timer: {}, time of creation {}", b, timer, self.time_of_creation)
            }
            true
        } else {
            false
        }
    }
}
