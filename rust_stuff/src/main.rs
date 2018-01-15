extern crate chrono;
extern crate serialport;
extern crate serde_json;
#[macro_use]
extern crate itertools;
use chrono::prelude::*;
use serde_json::{Value, Error};
use std::sync::{Arc, Mutex};
use chrono::Duration;
use serialport::prelude::*;
use std::io;
use std::time;
use itertools::Itertools;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::net::TcpStream;
use std::net::TcpListener;
use std::io::Read;
use std::str;

static PORT_NAME: &'static str = "/dev/ttyUSB0";



fn main() {
    let t: DateTime<Local> = Local::now();
    let z = t + Duration::weeks(2) + Duration::days(2);
    println!("{:?}, {:?}", z.day(), z.month());

    init_threads();
        
}


fn init_threads(){
    // let (sender_to_redirect, receiver_of_redirect): (Sender<[u8]>, Receiver<[u8]>) = mpsc::channel();
    thread::sleep(time::Duration::from_millis(1000));
    let (sender_tcp_sorter, receiver_tcp_sorter): (Sender<[u8; 1000]>, Receiver<[u8; 1000]>) = mpsc::channel();
    let (sender_sorter_command_queue, receiver_sorter_command_queue): (Sender<Command>, Receiver<Command>) = mpsc::channel();
    let (sender_command_queue_to_executer, receiver_command_queue_to_executer): (Sender<(u8, u8)>, Receiver<(u8, u8)>) = mpsc::channel();

    let tcp_listener_thread = thread::Builder::new().name("tcp_listener".to_string()).spawn(move ||{
        tcp_listener(sender_tcp_sorter);
    });
    let socket_msg_redirect_thr = thread::Builder::new().name("sorter".to_string()).spawn(move || {
        socket_msg_redirect(receiver_tcp_sorter, sender_sorter_command_queue);
    });
    let command_queue_to_send_thr = thread::Builder::new().name("timer".to_string()).spawn(move ||{
        command_queue_to_send_thread(sender_command_queue_to_executer, receiver_sorter_command_queue);
    });
    let receive_arduino_commands_thr = thread::Builder::new().name("receive".to_string()).spawn(move ||{
        receive_arduino_commands(receiver_command_queue_to_executer);
    });
    
    for h in vec![tcp_listener_thread, receive_arduino_commands_thr, 
                  command_queue_to_send_thr, socket_msg_redirect_thr] {
        h.unwrap().join().unwrap();
    }
}


fn print_port_name() {
    let port_name = PORT_NAME;
    if let Ok(mut port) = serialport::open(port_name) {
        let wr_vec = vec![1, 124, 1].clone().into_iter().collect::<Vec<_>>();
        let write_result = port.write(&wr_vec[..]);

    } else{
        println!("Error: Port '{}' not available", port_name);
    }
}



fn send_arduino_commands(sender: Sender<(u8, u8)>, pin: u8, status: u8){
    thread::sleep(time::Duration::from_millis(2100));
    loop {
        sender.send((pin, status)).unwrap();
    }   
}


fn receive_arduino_commands(exec_reciever: Receiver<(u8, u8)>){
    if let Ok(mut port) = serialport::open(PORT_NAME) {
        thread::sleep(time::Duration::from_millis(2000));
        loop{
            match exec_reciever.try_recv() {
                Ok((pin, status)) => send_via_usb(&mut port, pin, status),
                _ => ()
            }
            thread::sleep(time::Duration::from_millis(10));
        }
    }else{
        ()
    }
}


fn tcp_listener(sender: Sender<[u8; 1000]>){
    let listener = TcpListener::bind("127.0.0.1:8001").unwrap();
    loop{
        let mut buffer = [0; 1000];
        match listener.accept() {
            Ok((mut socket, addr)) => 
                {
                    socket.read(&mut buffer[..]);
                    sender.send(buffer);
                },
            Err(e) => (),
        }   
    }
}


fn command_queue_to_send_thread(sender: Sender<(u8, u8) >, receiver: Receiver<Command>){
    let mut command_vec: Vec<Command> = vec![];
    loop{
        match receiver.try_recv() {
            Ok(i) => command_vec.push(i),
            _ => ()
        }
        command_vec = command_vec.into_iter()
                                 .filter(|i| !(i.ready_to_execute(&sender)))
                                 .collect::<Vec<Command>>();
    }
}


fn socket_msg_redirect(receiver: Receiver<[u8; 1000]>, sender: Sender<Command>){
    let sender = sender;
    loop{
        match receiver.try_recv() {
            Ok(buffer) => 
                {
                    let buffer = buffer.into_iter()
                                       .take_while(|i| **i >0)
                                       .map(|i| *i)
                                       .collect::<Vec<u8>>();
                    let buff: &str = std::str::from_utf8(&buffer).unwrap();
                    println!("{}", &buff);
                    match serde_json::from_str(buff) {
                        Ok(i) => {
                            let command: Command = Command::new(i);
                            sender.send(command).unwrap();
                        },
                        Err(i) => println!("{:?}", i)
                    }
                    
                },
            _ => ()
        }
    }
}


fn send_via_usb(port: &mut std::boxed::Box<serialport::SerialPort>, pin: u8, status: u8){
    let wr_vec = vec![pin, status].clone().into_iter().collect::<Vec<_>>();
    let write_result = port.write(&wr_vec[..]);
    let mut buffer = vec![0; 2];
    thread::sleep(time::Duration::from_millis(200));
}


struct Command {
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

    fn ready_to_execute(&self, sender: &Sender<(u8, u8)>) -> bool{
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
