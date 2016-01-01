//! Contains the WebSocket server component.

use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::sync::mpsc::channel;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, Thread};

use crossbeam;
use uuid::Uuid;
use websockets::{Message, Sender, Receiver, WebSocketStream};
use websockets::header::WebSocketProtocol;
use websockets::message::Type;
use websockets::server::Request;
use websockets::client;

/// The WebSocket server.
///
/// Manages WebSocket connections from clients of the HTTP server.
pub struct Server {
    /// Stores the last markdown received, so that we have something to send to new connections.
    last_markdown: Arc<RwLock<String>>,

    server: TcpListener,
}

impl Server {
    /// Creates a new server that listens on port `port`.
    pub fn new<A>(socket_addr: A) -> Server where A: ToSocketAddrs {
        Server {
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

        let last_markdown = self.last_markdown.clone();
        let server = self.server.try_clone().unwrap();
        thread::spawn(move || {
            for markdown in markdown_receiver.iter() {
                let markdown: String = markdown;
                ws_channel_sender.send(Message::text(markdown.to_owned())).unwrap();
            }
        });

        thread::spawn(move || {
            crossbeam::scope(|scope| {
                

                for stream in server.incoming() {
                    scope.spawn(move || {
                        let stream = WebSocketStream::Tcp(stream.unwrap());
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
                        let (ws_sender, ws_receiver) = client.split();

                        let (ws_channel_sender, ws_channel_receiver) = mpsc::channel();

                        crossbeam::scope(|scope| {
                            scope.spawn(move || {
                            });
                        });

                        crossbeam::scope(|scope| {
                            scope.spawn(move || {
                                // for message in message_receiver.iter() {
                                //     let message: Message = message;
                                //     ws_sender.send_message(&message).unwrap();
                                // }
                            });
                        });
                    });
                }
            });

                // // Create the send and receive channdels for the websocket.
                // let (message_sender, message_receiver) = mpsc::channel();

                // // let initial_markdown = last_markdown.read().unwrap().to_owned();
                // // md_tx.send(initial_markdown).unwrap();

                // // WebSocket Message receiver
                // let ws_message_sender = message_sender.clone();
                // let receiver = thread::spawn(move || {
                //     // Message sender
                //     for markdown in markdown_receiver.iter() {
                //         let markdown: String = markdown;
                //         ws_message_sender.send(Message::text(markdown.to_owned())).unwrap();
                //     }
                // });

                // // WebSocket Message sender
                // let handle = thread::spawn(move || {
                //     for message in message_receiver.iter() {
                //         let message: Message = message;
                //         ws_sender.send_message(&message).unwrap();
                //     }
                // });

                // for message in ws_receiver.incoming_messages() {
                //     let message: Message = match message {
                //         Ok(m) => m,
                //         Err(_) => Message::close(),
                //     };

                //     let message = match message.opcode {
                //         Type::Close => Message::close(),
                //         Type::Ping => Message::pong(message.payload),
                //         _ => message,
                //     };

                //     message_sender.send(message).unwrap();
                // }
        });

        markdown_sender
    }
}
