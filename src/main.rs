extern crate termion;
extern crate chrono;

use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;
use termion::screen::*;

use std::io::{Write, stdout, stdin};
use std::io::prelude::*;
use std::net::TcpStream;

use std::thread::spawn;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};


use std::process;

use chrono::prelude::*;

#[derive(Debug)]
pub enum IRCMessage {
    Pass(String),
    Nick(String),
    User(String, String),
    Ping(String),
    Pong(String),
    Join(String),
    Part(String, String, String),
    Notice(String, String, String),
    PrivMsg(String, String, String),
    Quit(),
    Unknown(String),
    Nothing
}

impl IRCMessage {
    pub fn to_string(self) -> Result<String, String> {
        match self {
            IRCMessage::Pass(password) => {
                Ok(format!("PASS {}", password))
            },
            IRCMessage::Nick(name) => {
                Ok(format!("NICK {}", name))
            },
            IRCMessage::User(name, realname) => {
                Ok(format!("USER {} 0 * :{}", name, realname))
            },
            IRCMessage::Ping(data) => {
                Ok(format!("PING {}", data))
            },
            IRCMessage::Pong(data) => {
                Ok(format!("PONG {}", data))
            },
            IRCMessage::Join(channel) => {
                Ok(format!("JOIN {}", channel))
            },
            IRCMessage::Notice(_, target, message) => {
                Ok(format!("NOTICE {} {}", target, message))
            },
            IRCMessage::PrivMsg(_, target, message) => {
                Ok(format!("PRIVMSG {} {}", target, message))
            },
            IRCMessage::Part(_, channel, _) => {
                Ok(format!("PART {}", channel))
            }
            IRCMessage::Quit() => {
                Ok("QUIT".to_string())
            },
            IRCMessage::Unknown(data) => {
                Err(format!("{}", data))
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

        let cmd = cmd.unwrap();
        
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
                IRCMessage::Join(chan)
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

    let (receive_write, receive_read): (Sender<IRCMessage>, Receiver<IRCMessage>) = channel();
    let (sender_write, sender_read): (Sender<IRCMessage>, Receiver<IRCMessage>) = channel();

    let senderw_receive_thread = sender_write.clone();
    let senderw_stdin_thread = sender_write.clone();
    
    let mut swriter = TcpStream::connect("irc.mozilla.org:6667").unwrap();
    let mut sreader = swriter.try_clone().unwrap();

    let _sender_thread = spawn(move || {
        loop {
            let message = sender_read.recv().expect("Reading from sender_read failed.");
            match message.to_string() {
                Ok(msg) => {
                    swriter.write(format!("{}\r\n", msg).as_bytes());
                },
                Err(e) => {
                    println!("Failed to send data: {}", e);
                }    
            }
        }
    });

    let _receive_thread = spawn(move || {        
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
                            IRCMessage::Nothing => {},
                            IRCMessage::Ping(server) => {
                                let pong_msg = IRCMessage::Pong(server);
                                senderw_receive_thread.send(pong_msg);
                            },
                            ircmessage => {
                                receive_write.send(ircmessage);
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

    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    
    let stdout_main = Arc::new(Mutex::new(screen));;
    let stdout_thread = stdout_main.clone();

    let _stdin_thread = spawn(move|| { 
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
                            senderw_stdin_thread.send(IRCMessage::Quit());
                            process::exit(0);
                        },
                        //TODO REMOVE testing keys
                        Key::Ctrl('o') => {
                            senderw_stdin_thread.send(IRCMessage::Join("#prueba".to_string()));
                        },
                        //TODO REMOVE testing keys
                        Key::Ctrl('m') => {
                            senderw_stdin_thread.send(IRCMessage::PrivMsg("".to_string(), "#prueba".to_string(), "hola mundo".to_string()));
                        },
                        _ => {
                            
                        }
                    }                    
                }
            }
        }
    });
    
    sender_write.send(IRCMessage::Pass("none".to_string()));
    sender_write.send(IRCMessage::Nick("unbalancedpare".to_string()));
    sender_write.send(IRCMessage::User("unbalancedparentheses".to_string(), "Federico Carrone".to_string()));
    

    loop {        
        match receive_read.recv() {
            Ok(msg) =>
                match msg.to_string() {
                    Ok(s) => {
                        let now: DateTime<Local> = Local::now();
                        write!(stdout_main.lock().unwrap(), "{:02}:{:02} {}\r\n", now.hour(), now.minute(), s);
                        stdout_main.lock().unwrap().flush().unwrap();
                    },
                    Err(e) => {
                        write!(stdout_main.lock().unwrap(), "{}", e);
                    }
                },
            Err(RecvError) => {
                process::exit(1); //TODO this means that the socket was disconnected or the thread died
            }
        }
    }

    //TODO check that threads joined
    //sender_thread.join();
    //receive_thread.join();
    //stdin_thread.join();
}
