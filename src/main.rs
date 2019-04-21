extern crate termion;
use termion::{color, style};
use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;

use std::io::{Write, stdout, stdin};
use std::io::prelude::*;
use std::net::TcpStream;

use std::thread::spawn;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

use std::process;

#[derive(Debug)]
pub enum IRCMessage {
    Nick(String),
    User(String, String),
    Ping(String),
    Pong(String),
    Join(String, String),
    Unknown(String),
    Nothing
}

impl IRCMessage {
    pub fn to_string(self) -> Result<String, String> {
        match self {
            IRCMessage::Nick(name) => {
                Ok(format!("NICK {}\r\n", name))
            },
            IRCMessage::User(name, realname) => {
                Ok(format!("USER {} 0 * :{}\r\n", name, realname))
            },
            IRCMessage::Ping(data) => {
                Ok(format!("PING {}\r\n", data))
            },
            IRCMessage::Pong(data) => {
                Ok(format!("PONG {}\r\n", data))
            },
            IRCMessage::Join(_, channel) => {
                Ok(format!("JOIN {}\r\n", channel))
            },
            IRCMessage::Unknown(data) => {
                Err(data)
            },
            IRCMessage::Nothing => {
                Err("".to_string())
            }
        }
    }

    pub fn from_string(s: &str) -> Self {
        // println!("{}", s);
        let mut words = s.split_whitespace();
        return IRCMessage::Nothing;
    }
}

fn main() {

    let (receive_write, receive_read): (Sender<String>, Receiver<String>) = channel();
    let (sender_write, sender_read): (Sender<String>, Receiver<String>) = channel();
    let (keys_write, keys_read): (Sender<Key>, Receiver<Key>) = channel();
    
    let mut swriter = TcpStream::connect("irc.mozilla.org:6667").unwrap();
    let mut sreader = swriter.try_clone().unwrap();

    let sender_thread = spawn(move || {
        loop {
            let message = sender_read.recv().expect("Reading from sender_read failed.");
            swriter.write(message.as_bytes());
        }
    });
    
    let receive_thread = spawn(move || {        
        'read: loop {
            let mut buffer = [0 as u8; 65535]; // using 6 byte buffer
            match sreader.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break 'read;
                    }
                    
                    let slice = &buffer[0 .. bytes_read];

                    let lines = String::from_utf8_lossy(&slice);
                    let broken_lines = lines.split("\n");
                    for message in broken_lines {
                        match IRCMessage::from_string(message) {
                            IRCMessage::Ping(data) => {
                                sender_write.send(IRCMessage::Pong().to_string());
                            },
                            _ => {   
                                receive_write.send(message.to_string()).unwrap();
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to receive data: {}", e);
                }
            }
        }
    });

        
    let stdout_main = Arc::new(Mutex::new(stdout().into_raw_mode().unwrap()));;
    let stdout_thread = stdout_main.clone();

    let stdin_thread = spawn(move|| { 
        loop {
            for key in stdin().keys() {
                if key.is_ok() {
                    match key.unwrap() {
                        Key::Char('\n') => {
                            write!(stdout_thread.lock().unwrap(), "\r\n");
                        },
                        Key::Char(c) => {
                            write!(stdout_thread.lock().unwrap(), "{}", c);
                            stdout_thread.lock().unwrap().flush().unwrap();
                        },
                        Key::Ctrl('c') => {
                            process::exit(0);
                        },
                        _ => {
                            
                        }
                    }                    
                }
            }
        }
    });

    sender_write.send(String::from("PASS none\n"));
    sender_write.send(String::from("NICK ertwiop\n"));
    sender_write.send(String::from("USER ertwiop blah blah blah\n"));
    sender_write.send(String::from("JOIN #archlinux\n"));

    loop {        
        let msg = receive_read.recv().unwrap();
            write!(stdout_main.lock().unwrap(), "{}\r\n", msg);        
        stdout_main.lock().unwrap().flush().unwrap();
    }
    
    sender_thread.join();
    receive_thread.join();
    stdin_thread.join();
}
