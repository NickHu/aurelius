//! Contains the WebSocket server component.

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::mpsc::channel;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use uuid::Uuid;
use websockets::server::Request;
use websockets::server::Connection;
use websockets::WebSocketStream;
use websockets::Server as WebSocketServer;
use websockets::{Message, Sender, Receiver};
use websockets::header::WebSocketProtocol;
use websockets::message::Type;

use crossbeam;

/// The WebSocket server.
///
/// Manages WebSocket connections from clients of the HTTP server.
pub struct Server {
    active_connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<String>>>>,

    /// Stores the last markdown received, so that we have something to send to new connections.
    last_markdown: Arc<RwLock<String>>,

    server: TcpListener,
}

impl Server {
    /// Creates a new server that listens on port `port`.
    pub fn new<A>(socket_addr: A) -> Server where A: ToSocketAddrs {
        Server {
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            last_markdown: Arc::new(RwLock::new(String::new())),
            server: TcpListener::bind(socket_addr).unwrap(),
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.server.local_addr()
    }

    /// Starts the server.
    ///
    /// This method does not return.
    pub fn start(&mut self) -> mpsc::Sender<String> {
        let (markdown_sender, markdown_receiver) = mpsc::channel();

        let last_markdown_lock = self.last_markdown.clone();
        let active_connections = self.active_connections.clone();
        thread::spawn(move || {
            for markdown in markdown_receiver.iter() {
                {
                    let mut last_markdown = last_markdown_lock.write().unwrap();
                    *last_markdown = markdown;
                }

                for (uuid, sender) in active_connections.lock().unwrap().iter_mut() {
                    debug!("notifying websocket {}", uuid);
                    sender.send(last_markdown_lock.read().unwrap().to_owned()).unwrap();
                }
            }
        });

        let active_connections = self.active_connections.clone();
        let last_markdown = self.last_markdown.clone();
        let server = self.server.try_clone().unwrap();
        thread::spawn(move || {
            for stream in server.incoming() {
                // Spawn a new thread for each new connection.
                let stream = WebSocketStream::Tcp(stream.unwrap());
                let active_connections = active_connections.clone();
                let last_markdown = last_markdown.clone();
                thread::spawn(move || {
                    let request = Request::read(stream.try_clone().unwrap(), stream.try_clone().unwrap()).unwrap();
                    let headers = request.headers.clone();

                    request.validate().unwrap();

                    let mut response = request.accept();

                    if let Some(&WebSocketProtocol(ref protocols)) = headers.get() {
                        if protocols.contains(&("rust-websocket".to_string())) {
                            response.headers.set(WebSocketProtocol(vec!["rust-websocket".to_string()]));
                        }
                    }

                    let client = response.send().unwrap();

                    // Create the send and recieve channdels for the websocket.
                    let (mut sender, mut receiver) = client.split();

                    // Create senders that will send markdown between threads.
                    let (message_tx, message_rx) = channel();
                    let (md_tx, md_rx) = channel();

                    // Store the sender in the active connections.
                    let uuid = Uuid::new_v4();
                    active_connections.lock().unwrap().insert(uuid, md_tx.clone());

                    let initial_markdown = last_markdown.read().unwrap().to_owned();

                    md_tx.send(initial_markdown).unwrap();

                    // Message receiver
                    let ws_message_tx = message_tx.clone();
                    let receive_loop = thread::spawn(move || {
                        for message in receiver.incoming_messages() {
                            let message: Message = match message {
                                Ok(m) => m,
                                Err(_) => {
                                    let _ = ws_message_tx.send(Message::close());
                                    return;
                                }
                            };

                            match message.opcode {
                                Type::Close => {
                                    let message = Message::close();
                                    ws_message_tx.send(message).unwrap();
                                    return;
                                },
                                Type::Ping => {
                                    let message = Message::pong(message.payload);
                                    ws_message_tx.send(message).unwrap();
                                }
                                _ => ws_message_tx.send(message).unwrap(),
                            }
                        }
                    });

                    let send_loop = thread::spawn(move || {
                        for message in message_rx.iter() {
                            let message: Message = message;
                            sender.send_message(&message).unwrap();
                        }
                    });

                    for markdown in md_rx.iter() {
                        message_tx.send(Message::text(markdown)).unwrap();
                    }

                    let _ = send_loop.join();
                    let _ = receive_loop.join();
                });
            }
        });

        markdown_sender
    }
}
