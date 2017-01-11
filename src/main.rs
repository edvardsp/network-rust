
extern crate rand;
extern crate network_rust;

use std::thread;
use std::sync::mpsc::channel;

use network_rust::localip::get_localip;
use network_rust::peer::{PeerTransmitter, PeerReceiver, PeerUpdate};

const NUM_TRANSMITTERS: u32 = 2;
const NUM_RECEIVERS: u32 = 1;

fn main() {
    let port = 9887;

    for i in 0..NUM_TRANSMITTERS {
        thread::spawn(move || {
            let id = format!("{}:{}", get_localip().unwrap(), i);
            let transmitter = PeerTransmitter::new(port).unwrap();
            transmitter.run(&id);
        });
    }
    let (tx, rx) = channel::<PeerUpdate<String>>();
    for _ in 0..NUM_RECEIVERS {
        let tx = tx.clone();
        thread::spawn(move|| {
            let receiver = PeerReceiver::new(port).unwrap();
            receiver.run(tx);
        });
    }
    for _ in 0..10 {
        let update = rx.recv().unwrap();
        println!("Peer update");
        println!("\tPeers:\t{:?}", update.get_peers());
        println!("\tNew:\t{:?}", update.get_new());
        println!("\tLost:\t{:?}", update.get_lost());
    }
}
