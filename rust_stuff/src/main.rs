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
mod delayed_command;
use delayed_command::Command;

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
    let send_arduino_commands_thr = thread::Builder::new().name("receive".to_string()).spawn(move ||{
        send_arduino_commands(receiver_command_queue_to_executer);
    });
    
    for h in vec![tcp_listener_thread, send_arduino_commands_thr, 
                  command_queue_to_send_thr, socket_msg_redirect_thr] {
        h.unwrap().join().unwrap();
    }
}


fn send_arduino_commands(exec_reciever: Receiver<(u8, u8)>){
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
    let sent_byte: u8 = (status << 7) + pin;
    let wr_vec = vec![sent_byte].clone().into_iter().collect::<Vec<_>>();
    let write_result = port.write(&wr_vec[..]);
    thread::sleep(time::Duration::from_millis(200));
}
