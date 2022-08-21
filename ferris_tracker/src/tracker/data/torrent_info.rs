use super::{constants::*, peer_info::PeerInfo};
use shared::parsers::bencoding::{self, values::ValuesBencoding};
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

    /// Devuelve un bool que indica si el peer a agregar es nuevo (true) o no (false, si ya estaba en el torrent)
    pub fn add_peer(&mut self, peer_id: Vec<u8>, peer_info: PeerInfo) -> bool {
        let mut is_new_peer = true;
        if self.peers.contains_key(&peer_id) {
            is_new_peer = false;
        }
        self.peers.insert(peer_id, peer_info);
        is_new_peer
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
        bencoding::encoder::from_dic(dic_to_bencode)
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
        bencoding::encoder::from_dic(dic_to_bencode)
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

#[cfg(test)]
mod tests_torrent_info {
    use super::*;
    use std::{net::SocketAddr, str::FromStr};

    use crate::ResultDyn;

    fn create_default_torrent_info_with_multiple_peers_info() -> ResultDyn<TorrentInfo> {
        let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
        let info_hash_str = "abcdefghijklmn123456";
        let peer_id_str = "ABCDEFGHIJKLMNOPQRS0";
        let ip_str = "127.0.0.1";
        let port_str = "6881";
        let uploaded_str = "0";
        let downloaded_str = "0";
        let left_str = "128";
        let compact_str = "1";
        let event_str = "started";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

        let peer_0 = match PeerInfo::new(get_announce_msg, initial_addr) {
            Ok(peer_info) => peer_info,
            Err(err) => return Err(Box::new(err)),
        };

        let peer_id_str = "ABCDEFGHIJKLMNOPQRS1";
        let downloaded_str = "64";
        let left_str = "64";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

        let peer_1 = match PeerInfo::new(get_announce_msg, initial_addr) {
            Ok(peer_info) => peer_info,
            Err(err) => return Err(Box::new(err)),
        };

        let peer_id_str = "ABCDEFGHIJKLMNOPQRS2";
        let compact_str = "0";
        let event_str = "stopped";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

        let peer_2 = match PeerInfo::new(get_announce_msg, initial_addr) {
            Ok(peer_info) => peer_info,
            Err(err) => return Err(Box::new(err)),
        };

        let peer_id_str = "ABCDEFGHIJKLMNOPQRS3";
        let port_str = "6883";
        let uploaded_str = "16";
        let downloaded_str = "128";
        let left_str = "0";
        let compact_str = "0";
        let event_str = "completed";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

        let peer_3 = match PeerInfo::new(get_announce_msg, initial_addr) {
            Ok(peer_info) => peer_info,
            Err(err) => return Err(Box::new(err)),
        };

        let mut torrent_info = TorrentInfo::new(info_hash_str.as_bytes().to_vec());
        torrent_info.add_peer(peer_0.get_peer_id(), peer_0);
        torrent_info.add_peer(peer_1.get_peer_id(), peer_1);
        torrent_info.add_peer(peer_2.get_peer_id(), peer_2);
        torrent_info.add_peer(peer_3.get_peer_id(), peer_3);

        Ok(torrent_info)
    }

    #[test]
    fn getting_response_to_peer_that_requires_compact_ok() -> ResultDyn<()> {
        let torrent_info = create_default_torrent_info_with_multiple_peers_info()?;
        let peer_id = "ABCDEFGHIJKLMNOPQRS1".as_bytes().to_vec();
        let is_compact = true;

        let decoded_result_dic = bencoding::decoder::to_dic(
            torrent_info.get_bencoded_response_for_announce(peer_id, is_compact),
        )?
        .0;

        assert!(decoded_result_dic.contains_key(&COMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INCOMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INTERVAL_BYTES.to_vec()));

        let peers_dic = decoded_result_dic.get(&("peers".as_bytes().to_vec()));
        assert!(matches!(peers_dic, Some(&ValuesBencoding::String(..))));

        Ok(())
    }

    #[test]
    fn getting_response_to_peer_that_requires_no_compact_ok() -> ResultDyn<()> {
        let torrent_info = create_default_torrent_info_with_multiple_peers_info()?;
        let peer_id = "ABCDEFGHIJKLMNOPQRS3".as_bytes().to_vec();
        let is_compact = false;

        let decoded_result_dic = bencoding::decoder::to_dic(
            torrent_info.get_bencoded_response_for_announce(peer_id, is_compact),
        )?
        .0;

        assert!(decoded_result_dic.contains_key(&COMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INCOMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INTERVAL_BYTES.to_vec()));

        let peers_dic = decoded_result_dic.get(&("peers".as_bytes().to_vec()));
        assert!(matches!(peers_dic, Some(&ValuesBencoding::List(..))));

        Ok(())
    }

    #[test]
    fn getting_response_to_peer_that_didnt_specify_compact_ok() -> ResultDyn<()> {
        let torrent_info = create_default_torrent_info_with_multiple_peers_info()?;
        let peer_id = "ABCDEFGHIJKLMNOPQRS3".as_bytes().to_vec();
        let is_compact = false; // Si bien aca se asume, asi es como se comporta la funcion is_compact de los PeerInfo tmb

        let decoded_result_dic = bencoding::decoder::to_dic(
            torrent_info.get_bencoded_response_for_announce(peer_id, is_compact),
        )?
        .0;

        assert!(decoded_result_dic.contains_key(&COMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INCOMPLETE_BYTES.to_vec()));
        assert!(decoded_result_dic.contains_key(&INTERVAL_BYTES.to_vec()));

        let peers_dic = decoded_result_dic.get(&("peers".as_bytes().to_vec()));
        assert!(matches!(peers_dic, Some(&ValuesBencoding::List(..))));

        Ok(())
    }
}
