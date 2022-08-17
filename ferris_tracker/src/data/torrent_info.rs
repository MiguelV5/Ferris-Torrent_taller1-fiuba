use super::{
    super::bencoding::{encoder::from_dic, values::ValuesBencoding},
    constants::*,
    peer_info::PeerInfo,
};
use std::collections::HashMap;

pub struct TorrentInfo {
    info_hash: Vec<u8>,
    interval: i64,
    peers: HashMap<Vec<u8>, PeerInfo>,
}

impl TorrentInfo {
    pub fn new(info_hash: Vec<u8>) -> Self {
        let peers = HashMap::new();
        let interval = 0;

        TorrentInfo {
            info_hash,
            interval,
            peers,
        }
    }

    pub fn get_info_hash(&self) -> Vec<u8> {
        self.info_hash.clone()
    }

    pub fn add_peer(&mut self, peer_id: Vec<u8>, peer_info: PeerInfo) {
        self.peers.insert(peer_id, peer_info);
    }

    fn get_number_of_complete_and_incomplete_peers(&self) -> (i64, i64) {
        let mut complete = 0;
        let mut incomplete = 0;
        for peer in self.peers.values() {
            if peer.is_complete() {
                complete += 1;
            } else {
                incomplete += 1;
            }
        }
        (complete, incomplete)
    }

    fn get_response_no_compact(&self, peer_id: Vec<u8>) -> Vec<u8> {
        let (complete, incomplete) = self.get_number_of_complete_and_incomplete_peers();

        let mut dic_to_bencode: HashMap<Vec<u8>, ValuesBencoding> = HashMap::new();
        let mut list_peers: Vec<ValuesBencoding> = vec![];

        dic_to_bencode.insert(COMPLETE_BYTES.to_vec(), ValuesBencoding::Integer(complete));
        dic_to_bencode.insert(
            INCOMPLETE_BYTES.to_vec(),
            ValuesBencoding::Integer(incomplete),
        );
        dic_to_bencode.insert(
            INTERVAL_BYTES.to_vec(),
            ValuesBencoding::Integer(self.interval),
        );

        for key in self.peers.keys() {
            if key.clone() == peer_id {
                continue;
            }
            if let Some(peer_info) = self.peers.get(key) {
                if peer_info.is_stopped() {
                    continue;
                }
                let sock_addr = peer_info.get_sock_addr();
                let peer_id = key.clone();
                let ip = sock_addr.ip().to_string().as_bytes().to_vec();
                let port = sock_addr.port() as i64;

                let mut dic_peer = HashMap::new();
                dic_peer.insert(PEER_ID_BYTES.to_vec(), ValuesBencoding::String(peer_id));
                dic_peer.insert(IP_BYTES.to_vec(), ValuesBencoding::String(ip));
                dic_peer.insert(PORT_BYTES.to_vec(), ValuesBencoding::Integer(port));

                list_peers.push(ValuesBencoding::Dic(dic_peer))
            }
        }
        dic_to_bencode.insert(PEERS_BYTES.to_vec(), ValuesBencoding::List(list_peers));
        from_dic(dic_to_bencode)
    }

    fn get_response_compact(&self, peer_id: Vec<u8>) -> Vec<u8> {
        let (complete, incomplete) = self.get_number_of_complete_and_incomplete_peers();

        let mut dic_to_bencode: HashMap<Vec<u8>, ValuesBencoding> = HashMap::new();
        let mut vec_u8_peers = vec![];

        dic_to_bencode.insert(COMPLETE_BYTES.to_vec(), ValuesBencoding::Integer(complete));
        dic_to_bencode.insert(
            INCOMPLETE_BYTES.to_vec(),
            ValuesBencoding::Integer(incomplete),
        );
        dic_to_bencode.insert(
            INTERVAL_BYTES.to_vec(),
            ValuesBencoding::Integer(self.interval),
        );

        for key in self.peers.keys() {
            if key.clone() == peer_id {
                continue;
            }
            if let Some(peer_info) = self.peers.get(key) {
                if peer_info.is_stopped() {
                    continue;
                }
                let sock_addr = peer_info.get_sock_addr();
                for ip_num in sock_addr.ip().to_string().split('.') {
                    if let Ok(ip_num) = ip_num.parse::<u8>() {
                        vec_u8_peers.push(ip_num);
                    };
                }
                if let Ok(port_num) = sock_addr.port().to_string().parse::<u16>() {
                    let first_port = port_num / 256;
                    let second_port = port_num % 256;
                    vec_u8_peers.push(first_port as u8);
                    vec_u8_peers.push(second_port as u8);
                }
            }
        }
        dic_to_bencode.insert(PEERS_BYTES.to_vec(), ValuesBencoding::String(vec_u8_peers));
        from_dic(dic_to_bencode)
    }

    //Devuelvo la respuesta en formato bencoding, pido la peer_id solicitante para no devolver la misma al
    //dar la respuesta ya que puede que no sea la primera vez que se comunique y este incluido entre los peers.
    pub fn get_bencoded_response_for_announce(
        &self,
        peer_id: Vec<u8>,
        is_compact: bool,
    ) -> Vec<u8> {
        match is_compact {
            true => self.get_response_compact(peer_id),
            false => self.get_response_no_compact(peer_id),
        }
    }
}
