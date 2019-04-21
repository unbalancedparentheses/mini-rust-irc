extern crate termion;
extern crate chrono;

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

use std::time::SystemTime;
use chrono::prelude::*;

#[derive(Debug)]
pub enum IRCMessage {
    Nick(String),
    User(String, String),
    Ping(String),
    Pong(String),
    Join(String, String),
    Part(String, String, String),
    Notice(String, String, String),
    PrivMsg(String, String, String),
    Quit(String, String),
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
            IRCMessage::Notice(_, target, message) => {
                Ok(format!("NOTICE {} {}\r\n", target, message))
            },
            IRCMessage::PrivMsg(_, target, message) => {
                Ok(format!("PRIVMSG {} {}\r\n", target, message))
            },
            IRCMessage::Part(_, channel, _) => {
                Ok(format!("PART {}\r\n", channel))
            }
            IRCMessage::Quit(_, _) => {
                Ok("QUIT\r\n".to_string())
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
        let mut words = s.split_whitespace();

        let prefix = if s.starts_with(':') {
            words.next()
        } else {
            None
        };

        let source = prefix.unwrap_or("").split(':').nth(1).unwrap_or("").split("!").next().unwrap_or("").to_string();

        let cmd = words.next();
        if cmd.is_none() {
            return IRCMessage::Nothing;
        }

        let mut cmd = cmd.unwrap();

        match cmd {
            "NOTICE" => {
                let sender = words.next().unwrap().to_string();
                let rest: Vec<&str> = words.collect();
                let rest = rest.join(" ");
                IRCMessage::Notice(source, sender, rest)
            },
            "PRIVMSG" => {
                let sender = words.next().unwrap().to_string();
                let rest: Vec<&str> = words.collect();
                let rest = rest.join(" ").to_string();
                IRCMessage::PrivMsg(source, sender, rest)
            },
            "PING" => {
                let data: Vec<&str> = words.collect();
                let data = data.join(" ").to_string();
                IRCMessage::Ping(data)
            },
            "JOIN" => {
                let chan = words.next().unwrap().to_string();
                IRCMessage::Join(source, chan)
            },
            "PART" => {
                let chan = words.next().unwrap().to_string();
                let message: Vec<&str> = words.collect();
                let message = message.join(" ");
                IRCMessage::Part(source, chan, message)
            },
            _ => {
                let data: Vec<&str> = words.collect();
                let data = data.join(" ").to_string();
                IRCMessage::Unknown(format!("{} {} {}", source, cmd, data))
            }
        }
    }
}

fn main() {

    let (receive_write, receive_read): (Sender<String>, Receiver<String>) = channel();
    let (sender_write, sender_read): (Sender<String>, Receiver<String>) = channel();
    let (keys_write, keys_read): (Sender<Key>, Receiver<Key>) = channel();

    let sender_write_thread = sender_write.clone();
    
    let mut swriter = TcpStream::connect("irc.mozilla.org:6667").unwrap();
    let mut sreader = swriter.try_clone().unwrap();

    let sender_thread = spawn(move || {
        loop {
            let message = sender_read.recv().expect("Reading from sender_read failed.");
            swriter.write(message.as_bytes());
        }
    });

    let receive_thread = spawn(move || {        
        loop {
            let mut buffer = [0 as u8; 65535];
            match sreader.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }
                    
                    let slice = &buffer[0 .. bytes_read];

                    let lines = String::from_utf8_lossy(&slice);
                    let broken_lines = lines.split("\n");
                    for message in broken_lines {
                        match IRCMessage::from_string(message) {
                            IRCMessage::Ping(server) => {
                                let pong_msg = IRCMessage::Pong(server).to_string().unwrap();
                                sender_write_thread.send(pong_msg);
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
        let now: DateTime<Local> = Local::now();
        
        write!(stdout_main.lock().unwrap(), "{}:{} {}\r\n", now.hour(), now.minute(), msg);        
        stdout_main.lock().unwrap().flush().unwrap();
    }
    
    sender_thread.join();
    receive_thread.join();
    stdin_thread.join();
}
