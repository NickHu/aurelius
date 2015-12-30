extern crate aurelius;
extern crate websocket;

use std::io::prelude::*;

use aurelius::Server;
use websocket::{Client, Message, Receiver};
use websocket::client::request::Url;

#[test]
fn test_initial_send() {
    let mut server = Server::new();
    let sender = server.start();
    let url = Url::parse(&format!("ws://0.0.0.0:{}", server.websocket_addr().unwrap().port())).unwrap();

    let request = Client::connect(&url).unwrap();
    let response = request.send().unwrap();
    response.validate().unwrap();
    let (_, mut receiver) = response.begin().split();

    sender.send("Hello world!".to_owned()).unwrap();

    let message: Message = receiver.recv_message().unwrap();
    assert_eq!(String::from_utf8(message.payload.into_owned()).unwrap(), "<p>Hello world!</p>\n");
}

#[test]
fn test_multiple_send() {
    let mut server = Server::new();
    let sender = server.start();
    let url = Url::parse(&format!("ws://0.0.0.0:{}", server.websocket_addr().unwrap().port())).unwrap();

    let request = Client::connect(&url).unwrap();
    let response = request.send().unwrap();
    response.validate().unwrap();
    let (_, mut receiver) = response.begin().split();
    let mut messages = receiver.incoming_messages();

    sender.send("# Hello world!".to_owned()).unwrap();
    let hello_message: Message = messages.next().unwrap().unwrap();
    assert_eq!(String::from_utf8(hello_message.payload.into_owned()).unwrap(), "<h1>Hello world!</h1>\n");

    sender.send("# Goodbye world!".to_owned()).unwrap();
    let goodbye_message: Message = messages.next().unwrap().unwrap();
    assert_eq!(String::from_utf8(goodbye_message.payload.into_owned()).unwrap(), "<h1>Goodbye world!</h1>\n");
}
