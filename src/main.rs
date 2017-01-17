
#![feature(mpsc_select)]

#[macro_use]
extern crate serde_derive;
extern crate rand;
extern crate chrono;

use std::io;
use std::thread;
use std::sync::mpsc::channel;

use rand::Rng;
use chrono::offset::local::Local;

extern crate network_rust;
use network_rust::localip::get_localip;
use network_rust::peer::{PeerTransmitter, PeerReceiver, PeerUpdate};
use network_rust::bcast::{BcastTransmitter, BcastReceiver};

const PEER_PORT: u16 = 9877;
const BCAST_PORT: u16 = 9876;

#[derive(Serialize, Deserialize, Debug)]
struct MyPacket {
    msg: String,
    timestamp: i64,
}

fn main() {
    let unique = rand::thread_rng().gen::<u16>();

    // Spawn peer transmitter and receiver
    thread::spawn(move || {
        let id = format!("{}:{}", get_localip().unwrap(), unique);
        PeerTransmitter::new(PEER_PORT)
            .expect("Error creating PeerTransmitter")
            .run(&id);
    });
    let (peer_tx, peer_rx) = channel::<PeerUpdate<String>>();
    thread::spawn(move|| { 
        PeerReceiver::new(PEER_PORT)
            .expect("Error creating PeerReceiver")
            .run(peer_tx);
    });

    // Spawn broadcast transmitter and receiver
    let (transmit_tx, transmit_rx) = channel::<MyPacket>();
    let (receive_tx, receive_rx) = channel::<MyPacket>();
    thread::spawn(move|| {
        BcastTransmitter::new(BCAST_PORT)
            .expect("Error creating ")
            .run(transmit_rx);
    });
    thread::spawn(move|| {
        BcastReceiver::new(BCAST_PORT)
            .expect("Error creating BcastReceiver")
            .run(receive_tx);
    });

    // Spawn user interface
    thread::spawn(move|| {
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input)
                .expect("Couldn't read line");
            transmit_tx.send(MyPacket {
                msg:       input.trim().to_string(),
                timestamp: Local::now().timestamp(),
            }).unwrap();
        }
    });

    // Start infinite loop waiting on either bcast msg or peer update
    loop {
        select! {
            update = peer_rx.recv() => {
                println!("{}", update.unwrap());
            },
            bcast_msg = receive_rx.recv() => {
                println!("Got bcast_msg: {:?}", bcast_msg.unwrap());
            }
        }
    }
}
