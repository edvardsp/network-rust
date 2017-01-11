
use std::io;
use std::thread;
use std::net::UdpSocket;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::str::from_utf8;
use std::collections::HashMap;
use std::hash::Hash;

use serde;
use serde_json;

const INTERVAL_NS: u32 = 15_000_000; // 15 ms
const TIMEOUT_NS: u32 = 100_000_000; // 100 ms

pub struct PeerUpdate<T> {
    peers: Vec<T>,
    new: Option<T>,
    lost: Vec<T>,
}

impl<T> PeerUpdate<T> 
    where T: Ord,
{
    pub fn new() -> Self {
        PeerUpdate {
            peers: Vec::new(),
            new: None,
            lost: Vec::new(),
        }
    }

    pub fn add_peers(&mut self, id: T) {
        self.peers.push(id);
    }

    pub fn set_new(&mut self, id: T) {
        self.new = Some(id);
    }

    pub fn add_lost(&mut self, id: T) {
        self.lost.push(id);
    }

    pub fn sort(&mut self) {
        self.peers.sort();
        self.lost.sort()
    }

    pub fn get_peers(&self) -> &[T] {
        &self.peers[..]
    } 

    pub fn get_new(&self) -> Option<&T> {
        self.new.as_ref()
    } 

    pub fn get_lost(&self) -> &[T] {
        &self.lost[..]
    } 
}

pub struct PeerTransmitter {
    conn: UdpSocket,
    enabled: Mutex<bool>,
}

impl PeerTransmitter {
    pub fn new(port: u16) -> io::Result<Self> {
        let conn = {
            let socket = try!(UdpSocket::bind(("127.0.0.1", port)));
            try!(socket.set_broadcast(true));
            try!(socket.connect(("255.255.255.255", port)));
            socket
        };
        Ok(PeerTransmitter {
            conn: conn,
            enabled: Mutex::new(true),
        })
    }

    pub fn enable(&self) {
        let mut enabled = self.enabled.lock().unwrap();
        *enabled = true;
    }

    pub fn disable(&self) {
        let mut enabled = self.enabled.lock().unwrap();
        *enabled = false;
    }

    pub fn transmit<'a, T>(&self, data: &'a T) -> io::Result<()> 
        where T: serde::ser::Serialize,
    {
        let serialized = serde_json::to_string(&data).unwrap();
        try!(self.conn.send(serialized.as_bytes()));
        Ok(())
    }

    pub fn run<'a, T>(&self, data: &'a T) 
        where T: serde::ser::Serialize,
    {
        loop {
            thread::sleep(Duration::new(0, INTERVAL_NS));
            let enabled = self.enabled.lock().unwrap();
            if !*enabled {
                continue;
            }
            drop(enabled);
            self.transmit(data).expect("Transmission of data failed for PeerTransmitter");
        }
    }
}


pub struct PeerReceiver {
    conn: UdpSocket,
}

impl PeerReceiver {
    pub fn new(port: u16) -> io::Result<Self> {
        let conn = try!(UdpSocket::bind(("255.255.255.255", port)));
        Ok(PeerReceiver{
            conn: conn,
        })
    }

    pub fn receive<T>(&self) -> io::Result<T>
        where T: serde::de::Deserialize, 
    {
        let mut buf = [0; 128];
        let (amt, _) = try!(self.conn.recv_from(&mut buf));
        let msg = from_utf8(&buf[..amt]).unwrap();
        Ok(serde_json::from_str(&msg).unwrap())
    }

    pub fn run<T>(&self, update_tx: mpsc::Sender<PeerUpdate<T>>)
        where T: serde::de::Deserialize + Hash + Eq + Clone + Ord,
    {
        let mut last_seen = HashMap::new();
        loop {
            let mut peer_update = PeerUpdate::new();
            let mut updated = false;
            
            self.conn.set_read_timeout(Some(Duration::new(0, TIMEOUT_NS))).unwrap();
            let new_id: T = match self.receive() {
                Ok(id) => id,
                Err(_) => {
                    println!("PeerReceiver timed out");
                    continue;
                }
            };
            
            // Adding new connection
            if !last_seen.contains_key(&new_id) {
                peer_update.set_new(new_id.clone());
                updated = true;
            }
            last_seen.insert(new_id, Instant::now());

            // Removing dead connection
            for (id, time) in &last_seen {
                if Instant::now().duration_since(*time) > Duration::new(0, TIMEOUT_NS) {
                    updated = true;
                    peer_update.add_lost(id.clone());
                }
            }

            // Sending update
            if updated {
                for (id, _) in &last_seen {
                    peer_update.add_peers(id.clone());
                }
                peer_update.sort();
                update_tx.send(peer_update).unwrap();
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::mpsc::channel;
    use localip::get_localip;

    #[test]
    fn it_works() {
        let port = 9887;
        thread::spawn(move || {
            let id = format!("{}:{}", get_localip().unwrap(), "unique");
            let transmitter = PeerTransmitter::new(port).unwrap();
            transmitter.run(&id);
        });
        let (tx, rx) = channel::<PeerUpdate<String>>();
        thread::spawn(move|| {
            let receiver = PeerReceiver::new(port).unwrap();
            receiver.run(tx);
        });
        for _ in 0..10 {
            let update = rx.recv().unwrap();
            println!("Peer update");
            println!("\tPeers:\t{:?}", update.peers);
            println!("\tNew:\t{:?}", update.new);
            println!("\tLost:\t{:?}", update.lost);
        }
    }
}

