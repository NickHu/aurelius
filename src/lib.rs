//! [aurelius](https://github.com/euclio/aurelius) is a complete solution for rendering and
//! previewing markdown.
//!
//! This crate provides a server that can render and update an HTML preview of markdown without a
//! client-side refresh. The server listens for both WebSocket and HTTP connections on arbitrary
//! ports. Upon receiving an HTTP request, the server renders a page containing a markdown preview.
//! Client-side JavaScript then initiates a WebSocket connection which allows the server to push
//! changes to the client.
//!
//! This crate was designed to power [vim-markdown-composer], a markdown preview plugin for
//! [Neovim](http://neovim.io), but it may be used to implement similar plugins for any editor.
//! See [vim-markdown-composer] for a usage example.
//!
//! aurelius follows stable Rust. However, the API currently unstable and may change without
//! warning.
//!
//! # Acknowledgments
//! This crate is inspired by suan's
//! [instant-markdown-d](https://github.com/suan/instant-markdown-d).
//!
//! # Why the name?
//! "Aurelius" is a Roman *gens* (family name) shared by many famous Romans, including emperor
//! Marcus Aurelius, one of the "Five Good Emperors." The gens itself originates from the Latin
//! *aureus* meaning "golden." Also, tell me that "Markdown Aurelius" isn't a great pun.
//!
//! <cite>[Aurelia (gens) on Wikipedia](https://en.wikipedia.org/wiki/Aurelia_(gens))</cite>.
//!
//! [vim-markdown-composer]: https://github.com/euclio/vim-markdown-composer

extern crate crossbeam;
extern crate hoedown;
extern crate porthole;
extern crate url;
extern crate uuid;
extern crate websocket as websockets;

#[macro_use]
extern crate log;
#[macro_use]
extern crate nickel;

pub mod browser;
pub mod markdown;

mod http;
mod websocket;

use std::net::SocketAddr;
use std::io;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{self, Sender};
use std::thread;

use http::Server as HttpServer;
use websocket::Server as WebSocketServer;


/// The `Server` type constructs a new markdown preview server.
///
/// The server will listen for HTTP and WebSocket connections on arbitrary ports.
pub struct Server<'a> {
    http_server: Arc<RwLock<HttpServer>>,
    websocket_server: WebSocketServer<'a>,
    initial_markdown: Option<String>,
    highlight_theme: Option<String>,
}

impl<'a> Server<'a> {
    /// Creates a new markdown preview server.
    ///
    /// Builder methods are provided to configure the server before starting it.
    pub fn new() -> Server<'a> {
        let http_port = porthole::open().unwrap();
        let http_server = HttpServer::new(http_port);

        Server {
            http_server: Arc::new(RwLock::new(http_server)),
            websocket_server: WebSocketServer::new(("localhost", 0)),
            initial_markdown: None,
            highlight_theme: None,
        }
    }

    pub fn websocket_addr(&self) -> io::Result<SocketAddr> {
        self.websocket_server.local_addr()
    }

    /// Starts the server, returning a `ServerHandle` to communicate with it.
    pub fn start(&mut self) -> Sender<String> {
        let websocket_server = &mut self.websocket_server;

        let (tx, rx) = mpsc::channel::<String>();

        // Start websocket server
        let markdown_receiver = Mutex::new(rx);

        crossbeam::scope(|scope| {
            scope.spawn(|| {
                let websocket_sender = websocket_server.start();

                for markdown in markdown_receiver.lock().unwrap().iter() {
                    let html: String = markdown::to_html(&markdown);
                    websocket_sender.send(html).unwrap();
                }
            });
        });

        let http_server = self.http_server.clone();

        let websocket_port = websocket_server.local_addr().unwrap().port();

        // Start http server
        let initial_markdown = match self.initial_markdown {
            Some(ref markdown) => markdown.clone(),
            None => "".to_string(),
        };
        let highlight_theme = match self.highlight_theme {
            Some(ref theme) => theme.clone(),
            None => "github".to_string(),
        };

        thread::spawn(move || {
            let server = http_server.read().unwrap();
            debug!("Starting http_server");
            server.start(websocket_port, initial_markdown, highlight_theme);
        });

        tx
    }
}
