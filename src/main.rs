
extern crate rand;
extern crate network_rust;

use std::thread;
use std::sync::mpsc::channel;
use std::process;

use rand::Rng;

use network_rust::localip::get_localip;
use network_rust::peer::{PeerTransmitter, PeerReceiver, PeerUpdate};

fn main() {
    let port = 9887;
    let unique = rand::thread_rng().gen::<u16>();

    thread::spawn(move || {
        let id = format!("{}:{}", get_localip().unwrap(), unique);
        let transmitter = match PeerTransmitter::new(port) {
            Ok(transmitter) => transmitter,
            Err(err) => {
                println!("Error creating peer transmitter, {}", err);
                process::exit(1);    
            }
        };
        transmitter.run(&id);
    });

    let (tx, rx) = channel::<PeerUpdate<String>>();
    thread::spawn(move|| { 
        let receiver = match PeerReceiver::new(port) {
            Ok(receiver) => receiver,
            Err(err) => {
                println!("Error creating peer receiver, {}", err);
                process::exit(1);
            }
        };
        receiver.run(tx);
    });

    loop {
        let update = rx.recv().unwrap();
        println!("{}", update);
    }
}
