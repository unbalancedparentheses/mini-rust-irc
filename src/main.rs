extern crate chrono;

use std::io;

use std::io::{Write, stdout};
use std::io::prelude::*;
use std::net::TcpStream;

use std::thread::spawn;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

use std::process;

use chrono::prelude::*;

extern crate tui;
extern crate termion;
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::Terminal;

extern crate rustyline;
use rustyline::error::ReadlineError;
use rustyline::Editor;

struct App {
    input: String,
    /// History of recorded messages
    messages: Vec<String>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            messages: Vec::new(),
        }
    }
}


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
            IRCMessage::Notice(_, target, message) => { //TODO check if we can have an arity 2 notice
                Ok(format!("NOTICE {} {}", target, message))
            },
            IRCMessage::PrivMsg(_, target, message) => { //TODO check if we can have an arity 2 privmsg
                Ok(format!("PRIVMSG {} {}", target, message))
            },
            IRCMessage::Part(_, channel, _) => { //TODO check if we can have an arity 1 part
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
                IRCMessage::Unknown(format!("{} {} {}\r\n", source, cmd, data))
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
    
    let mut rl = Editor::<()>::new();
    
    let _stdin_thread = spawn(move|| { 
        loop {
            let readline = rl.readline("");

            match readline {
                Ok(line) => {
                    match parse_commandline(line) {
                        Ok(message) => {
                            senderw_stdin_thread.send(message);
                        },
                        Err(e) => {
                            println!("Unknown command: {}", e);
                        }
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    senderw_stdin_thread.send(IRCMessage::Quit());
                    process::exit(0);                },
                Err(ReadlineError::Eof) => {
                    senderw_stdin_thread.send(IRCMessage::Quit());
                    process::exit(0);
                },
                Err(err) => {
                    println!("readline error: {:?}", err);
                }
            }
        }
    });
    
    sender_write.send(IRCMessage::Pass("none".to_string()));
    sender_write.send(IRCMessage::Nick("unbalancedpared".to_string()));
    sender_write.send(IRCMessage::User("unbalancedparentheses".to_string(), "Federico Carrone".to_string()));

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode().unwrap();
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Create default app state
    let mut app = App::default();
    
    loop {
        let (max_x, max_y) = termion::terminal_size().unwrap();

        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
                .split(f.size());

            let mut messages = app
                .messages
                .iter()
                .rev()
                .take(max_y as usize - 2)
                .map(|m| Text::raw(format!("{}\n", m)))
                .collect::<Vec<_>>();

            messages.reverse();

            Paragraph::new(messages.iter())
                .block(Block::default())
                .wrap(true)
                .render(&mut f, chunks[0]);

            Paragraph::new([Text::raw(&app.input)].iter())
                .style(Style::default().fg(Color::Green))
                .block(Block::default())
                .render(&mut f, chunks[1]);
        });

        // Put the cursor back inside the input box

        write!(
            terminal.backend_mut(),
            "{}",
            Goto(0, max_y - 1)
        );
        
        match receive_read.recv() {
            Ok(msg) =>
                match msg.to_string() {
                    Ok(s) => {  
                        let now: DateTime<Local> = Local::now();
                        let s_with_time = format!("{:02}:{:02} {}\r\n", now.hour(), now.minute(), s);
                        app.messages.push(s_with_time);
                    },
                    Err(e) => {
                        let now: DateTime<Local> = Local::now();
                        let s_with_time = format!("{:02}:{:02} Error: {}\r\n", now.hour(), now.minute(), e);
                        app.messages.push(s_with_time);
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

fn parse_commandline(s: String) -> Result<IRCMessage, String> {
    if s.starts_with('/') {
        let mut iter = s.split_whitespace();
        match iter.next() {
            Some("/join") => {
                match iter.next() {
                    Some(channel) => Ok(IRCMessage::Join(channel.to_string())),
                    None => Err("Not enough parameters given".to_string())
                }
            },
            Some(_) => {
                Err("Command not supported".to_string())
            }
            None => {
                Err("Not enough parameters given".to_string())
            }
        }
    } else {
        Ok(IRCMessage::PrivMsg("".to_string(), "#prueba".to_string(), s.to_string()))
    }
}
