use std::collections::{hash_map, HashMap};

use super::constants::*;

use gtk::prelude::*;
use gtk::{Align, Orientation};

fn create_default_box(orientation_box: Orientation) -> gtk::Box {
    gtk::Box::builder()
        .orientation(orientation_box)
        .spacing(FIVE)
        .margin_top(FIVE)
        .margin_bottom(FIVE)
        .margin_start(FIVE)
        .margin_end(FIVE)
        .build()
}

fn create_default_label(label: String) -> gtk::Label {
    gtk::Label::builder().label(label.as_str()).build()
}

fn create_default_stack() -> gtk::Stack {
    gtk::Stack::builder().hexpand(true).vexpand(true).build()
}

fn create_main_box() -> (gtk::Box, gtk::Stack) {
    //Stack contenedor de cada pestaña principal
    let stack_main = create_default_stack();
    stack_main.add_css_class(CSS_STACK_MAIN);

    //Las pestañas
    let switch_stack = gtk::StackSwitcher::builder().stack(&stack_main).build();

    //El box main del programa
    let box_main = create_default_box(Orientation::Vertical);

    box_main.append(&switch_stack);
    box_main.append(&stack_main);

    (box_main, stack_main)
}

fn create_box_for_stacks() -> (gtk::Box, gtk::Stack) {
    //El box contenedor de los stack de info y sus pestañas
    let box_for_stacks = create_default_box(Orientation::Horizontal);

    //Los stacks de info
    let stacks_info = create_default_stack();
    stacks_info.add_css_class(CSS_STACK_INFO);

    //Scroller y viewport para poder recorrer en caso de haber varias pestañas
    let scroller = gtk::ScrolledWindow::builder()
        .propagate_natural_width(true)
        .min_content_width(MIN_SCROLL_WIDTH)
        .build();
    let viewport = gtk::Viewport::builder().build();

    //Las pestañas de los distintos stacks de info
    let switcher_stacks_info = gtk::StackSwitcher::builder()
        .orientation(Orientation::Vertical)
        .halign(Align::Fill)
        .valign(Align::Start)
        .stack(&stacks_info)
        .build();

    viewport.set_child(Some(&switcher_stacks_info));
    scroller.set_child(Some(&viewport));

    box_for_stacks.append(&scroller);
    box_for_stacks.append(&stacks_info);

    (box_for_stacks, stacks_info)
}

struct InfoBox {
    info_box: gtk::Box,
    labels: HashMap<String, gtk::Label>,
    progress_bar: gtk::ProgressBar,
    image_ferris: gtk::Image,
    active_connections: u64,
}

impl InfoBox {
    fn new() -> Self {
        let progress_bar = gtk::ProgressBar::builder().show_text(true).build();
        let image_ferris = gtk::Image::builder().hexpand(true).vexpand(true).build();

        InfoBox {
            info_box: create_default_box(Orientation::Vertical),
            labels: HashMap::new(),
            progress_bar,
            image_ferris,
            active_connections: 0,
        }
    }

    fn get_box(&self) -> &gtk::Box {
        &self.info_box
    }

    fn init_labels_torrent(&mut self) {
        self.init_tracker(String::from(INCOGNITO));
        self.init_info_hash(String::from(INCOGNITO));
        self.init_total_size(ZERO);
        self.init_cant_pieces(ZERO);
        self.init_peers(ZERO as u64, ZERO as u64);
        self.init_single_multiple(String::from(INCOGNITO));
        self.init_pieces_downloaded(ZERO);
        self.init_active_connections();
        self.init_progress_bar(ZERO as u64);
    }

    fn init_tracker(&mut self, tracker: String) {
        let tracker_id = String::from(TRACKER_ID);

        let mut tracker_data = String::from(TRACKER_LABEL);
        tracker_data.push_str(tracker.as_str());

        let label_tracker = create_default_label(tracker_data);

        self.info_box.append(&label_tracker);
        self.labels.insert(tracker_id, label_tracker);
    }

    fn init_info_hash(&mut self, info_hash: String) {
        let info_hash_id = String::from(INFO_HASH_ID);

        let mut info_hash_data = String::from(INFO_HASH_LABEL);
        info_hash_data.push_str(info_hash.as_str());

        let label_info_hash = create_default_label(info_hash_data);

        self.info_box.append(&label_info_hash);
        self.labels.insert(info_hash_id, label_info_hash);
    }

    fn init_total_size(&mut self, size: u64) {
        let size_id = String::from(TOTAL_SIZE_ID);

        let mut total_size_data = String::from(TOTAL_SIZE_LABEL);
        total_size_data.push_str(size.to_string().as_str());

        let label_total_size = create_default_label(total_size_data);

        self.info_box.append(&label_total_size);
        self.labels.insert(size_id, label_total_size);
    }

    fn init_cant_pieces(&mut self, cant_pieces: u64) {
        let cant_pieces_id = String::from(CANT_PIECES_ID);

        let mut cant_pieces_data = String::from(CANT_PIECES_LABEL);
        cant_pieces_data.push_str(cant_pieces.to_string().as_str());

        let label_cant_pieces = create_default_label(cant_pieces_data);

        self.info_box.append(&label_cant_pieces);
        self.labels.insert(cant_pieces_id, label_cant_pieces);
    }

    fn init_peers(&mut self, peers: u64, leechers: u64) {
        let peers_id = String::from(PEERS_LEECHERS_ID);

        let mut peers_label = String::from(SEEDERS_LABEL);
        peers_label.push_str(peers.to_string().as_str());
        peers_label.push_str(LEECHERS_LABEL);
        peers_label.push_str(leechers.to_string().as_str());

        let label_info_peers = create_default_label(peers_label);

        self.info_box.append(&label_info_peers);
        self.labels.insert(peers_id, label_info_peers);
    }

    fn init_single_multiple(&mut self, type_torrent: String) {
        let single_multiple_id = String::from(SINGLE_MULTIPLE_ID);

        let single_multiple_data = String::from(type_torrent.as_str());

        let label_single_multiple = create_default_label(single_multiple_data);

        self.info_box.append(&label_single_multiple);
        self.labels
            .insert(single_multiple_id, label_single_multiple);
    }

    fn init_pieces_downloaded(&mut self, pieces_download: u64) {
        let pieces_downloaded_id = String::from(PIECES_DOWNLOADED_ID);

        let mut pieces_downloaded_data = String::from(PIECES_DOWNLOADED_LABEL);
        pieces_downloaded_data.push_str(pieces_download.to_string().as_str());

        let label_pieces_downloaded = create_default_label(pieces_downloaded_data);

        self.info_box.append(&label_pieces_downloaded);
        self.labels
            .insert(pieces_downloaded_id, label_pieces_downloaded);
    }

    fn init_active_connections(&mut self) {
        let active_connections_id = String::from(ACTIVE_CONNECTIONS_ID);

        let mut active_connections_data = String::from(ACTIVE_CONNECTIONS_LABEL);
        active_connections_data.push_str(self.active_connections.to_string().as_str());

        let label_active_connections = create_default_label(active_connections_data);

        self.info_box.append(&label_active_connections);
        self.labels
            .insert(active_connections_id, label_active_connections);
    }

    fn init_progress_bar(&mut self, progress: u64) {
        self.progress_bar.set_fraction(progress as f64);
        let dummy = create_default_label(String::new());
        self.image_ferris.set_from_file(Some(IMAGE_FERRIS));
        self.info_box.append(&dummy);
        self.info_box.append(&self.progress_bar);
        self.info_box.append(&self.image_ferris);
    }

    fn change_tracker(&mut self, tracker: String) {
        let tracker_label = self.labels.get(TRACKER_ID);
        match tracker_label {
            Some(label) => {
                let mut new_tracker = String::from(TRACKER_LABEL);
                new_tracker.push_str(tracker.as_str());
                label.set_label(&new_tracker)
            }
            None => (),
        }
    }

    fn change_info_hash(&mut self, info_hash: String) {
        let info_hash_label = self.labels.get(INFO_HASH_ID);
        match info_hash_label {
            Some(label) => {
                let mut new_info_hash = String::from(INFO_HASH_LABEL);
                new_info_hash.push_str(info_hash.as_str());
                label.set_label(&new_info_hash)
            }
            None => (),
        }
    }

    fn change_total_size(&mut self, total_size: u64) {
        let total_size_label = self.labels.get(TOTAL_SIZE_ID);
        match total_size_label {
            Some(label) => {
                let mut new_total_size = String::from(TOTAL_SIZE_LABEL);
                new_total_size.push_str(total_size.to_string().as_str());
                new_total_size.push_str(BYTES);
                label.set_label(&new_total_size)
            }
            None => (),
        }
    }

    fn change_cant_pieces(&mut self, cant_pieces: u64) {
        let cant_pieces_label = self.labels.get(CANT_PIECES_ID);
        match cant_pieces_label {
            Some(label) => {
                let mut new_cant_pieces = String::from(CANT_PIECES_LABEL);
                new_cant_pieces.push_str(cant_pieces.to_string().as_str());
                label.set_label(&new_cant_pieces)
            }
            None => (),
        }
    }

    fn change_peers_leechers(&mut self, seeders: u64, leechers: u64) {
        let peers_leechers_label = self.labels.get(PEERS_LEECHERS_ID);
        match peers_leechers_label {
            Some(label) => {
                let mut new_peers_leechers = String::from(SEEDERS_LABEL);
                new_peers_leechers.push_str(seeders.to_string().as_str());
                new_peers_leechers.push_str(LEECHERS_LABEL);
                new_peers_leechers.push_str(leechers.to_string().as_str());
                label.set_label(&new_peers_leechers)
            }
            None => (),
        }
    }

    fn change_single_multiple(&mut self, single_multiple: TorrentFileType) {
        let type_torrent = match single_multiple {
            TorrentFileType::SingleFile => SINGLE_FILE,
            TorrentFileType::MultipleFile => MULTIPLE_FILE,
        };
        let single_multiple_label = self.labels.get(SINGLE_MULTIPLE_ID);
        match single_multiple_label {
            Some(label) => {
                let new_single_multiple = String::from(type_torrent);
                label.set_label(&new_single_multiple)
            }
            None => (),
        }
    }

    fn change_pieces_downloaded(&mut self, pieces_downloaded: u64) {
        let pieces_downloaded_label = self.labels.get(PIECES_DOWNLOADED_ID);
        match pieces_downloaded_label {
            Some(label) => {
                let mut new_pieces_downloaded = String::from(PIECES_DOWNLOADED_LABEL);
                new_pieces_downloaded.push_str(pieces_downloaded.to_string().as_str());
                label.set_label(&new_pieces_downloaded)
            }
            None => (),
        }
    }

    fn change_active_connections(&mut self, type_of_change: TypeOfChange) {
        match type_of_change {
            TypeOfChange::Addition => self.active_connections += 1,
            TypeOfChange::Substraction => self.active_connections -= 1,
        }
        let active_connections_label = self.labels.get(ACTIVE_CONNECTIONS_ID);
        match active_connections_label {
            Some(label) => {
                let mut new_active_connections = String::from(ACTIVE_CONNECTIONS_LABEL);
                new_active_connections.push_str(self.active_connections.to_string().as_str());
                label.set_label(&new_active_connections)
            }
            None => (),
        }
    }

    fn change_progress_bar(&mut self, progress: f64) {
        if progress == ONE {
            self.image_ferris.set_from_file(Some(IMAGE_FERRIS3))
        }

        self.progress_bar.set_fraction(progress);
    }

    fn init_labels_peers(&mut self) {
        self.init_peer_id(String::from(INCOGNITO));
        self.init_ip(String::from(INCOGNITO));
        self.init_port(ZERO);
        self.init_download(ZERO as u64);
        self.init_upload(ZERO as u64);
        self.init_state_peer(String::from(INCOGNITO));
        self.init_state_client(String::from(INCOGNITO));
    }

    fn init_peer_id(&mut self, peer_id: String) {
        let peer_id_id = String::from(PEER_ID_ID);

        let mut peer_id_data = String::from(PEER_ID_LABEL);
        peer_id_data.push_str(peer_id.as_str());

        let label_peer_id = create_default_label(peer_id_data);

        self.info_box.append(&label_peer_id);
        self.labels.insert(peer_id_id, label_peer_id);
    }

    fn init_ip(&mut self, ip: String) {
        let ip_id = String::from(IP_ID);

        let mut ip_data = String::from(IP_LABEL);
        ip_data.push_str(ip.as_str());

        let label_ip = create_default_label(ip_data);

        self.info_box.append(&label_ip);
        self.labels.insert(ip_id, label_ip);
    }

    fn init_port(&mut self, port: u64) {
        let port_id = String::from(PORT_ID);

        let mut port_data = String::from(PORT_LABEL);
        port_data.push_str(port.to_string().as_str());

        let label_port = create_default_label(port_data);

        self.info_box.append(&label_port);
        self.labels.insert(port_id, label_port);
    }

    fn init_download(&mut self, download: u64) {
        let download_id = String::from(DOWNLOAD_ID);

        let mut download_data = String::from(DOWNLOAD_LABEL);
        download_data.push_str(download.to_string().as_str());

        let label_download = create_default_label(download_data);

        self.info_box.append(&label_download);
        self.labels.insert(download_id, label_download);
    }

    fn init_upload(&mut self, upload: u64) {
        let upload_id = String::from(UPLOAD_ID);

        let mut upload_data = String::from(UPLOAD_LABEL);
        upload_data.push_str(upload.to_string().as_str());

        let label_upload = create_default_label(upload_data);

        self.info_box.append(&label_upload);
        self.labels.insert(upload_id, label_upload);
    }

    fn init_state_peer(&mut self, state_peer: String) {
        let state_peer_id = String::from(STATE_PEER_ID);

        let mut state_peer_data = String::from(STATE_PEER_LABEL);
        state_peer_data.push_str(state_peer.as_str());

        let label_state_peer = create_default_label(state_peer_data);

        self.info_box.append(&label_state_peer);
        self.labels.insert(state_peer_id, label_state_peer);
    }

    fn init_state_client(&mut self, state_client: String) {
        let state_client_id = String::from(STATE_CLIENT_ID);

        let mut state_client_data = String::from(STATE_CLIENT_LABEL);
        state_client_data.push_str(state_client.as_str());

        let label_state_client = create_default_label(state_client_data);

        self.info_box.append(&label_state_client);
        self.labels.insert(state_client_id, label_state_client);

        let dummy = create_default_label(String::new());
        self.image_ferris.set_from_file(Some(IMAGE_FERRIS2));

        self.info_box.append(&dummy);
        self.info_box.append(&self.image_ferris);
    }

    fn change_peer_id(&mut self, peer_id: String) {
        let peer_id_label = self.labels.get(PEER_ID_ID);
        match peer_id_label {
            Some(label) => {
                let mut new_peer_id = String::from(PEER_ID_LABEL);
                new_peer_id.push_str(peer_id.as_str());
                label.set_label(&new_peer_id)
            }
            None => (),
        }
    }

    fn change_ip(&mut self, ip: String) {
        let ip_label = self.labels.get(IP_ID);
        match ip_label {
            Some(label) => {
                let mut new_ip = String::from(IP_LABEL);
                new_ip.push_str(ip.as_str());
                label.set_label(&new_ip)
            }
            None => (),
        }
    }

    fn change_port(&mut self, port: u64) {
        let port_label = self.labels.get(PORT_ID);
        match port_label {
            Some(label) => {
                let mut new_port = String::from(PORT_LABEL);
                new_port.push_str(port.to_string().as_str());
                label.set_label(&new_port)
            }
            None => (),
        }
    }

    fn change_download(&mut self, download: f64) {
        let download_label = self.labels.get(DOWNLOAD_ID);
        match download_label {
            Some(label) => {
                let download = (download * f64::from(100)).round() / f64::from(100);
                let mut new_download = String::from(DOWNLOAD_LABEL);
                new_download.push_str(download.to_string().as_str());
                new_download.push_str(BYTESPERSEC);
                label.set_label(&new_download)
            }
            None => (),
        }
    }

    fn change_upload(&mut self, upload: f64) {
        let upload_label = self.labels.get(UPLOAD_ID);
        match upload_label {
            Some(label) => {
                let upload = (upload * f64::from(100)).round() / f64::from(100);
                let mut new_upload = String::from(UPLOAD_LABEL);
                new_upload.push_str(upload.to_string().as_str());
                new_upload.push_str(BYTESPERSEC);
                label.set_label(&new_upload)
            }
            None => (),
        }
    }

    fn change_state_peer(&mut self, state_peer: State) {
        let state = match state_peer {
            State::ChokeInterested => format!("{} - ", CHOKE) + INTERESTED,
            State::UnchokeInterested => format!("{} - ", UNCHOKE) + INTERESTED,
            State::ChokeNotInterested => format!("{} - ", CHOKE) + NOT_INTERESTED,
            State::UnchokeNotInterested => format!("{} - ", UNCHOKE) + NOT_INTERESTED,
        };
        let state_peer_label = self.labels.get(STATE_PEER_ID);
        match state_peer_label {
            Some(label) => {
                let mut new_state_peer = String::from(STATE_PEER_LABEL);

                new_state_peer.push_str(&state);
                label.set_label(&new_state_peer)
            }
            None => (),
        }
    }

    fn change_state_client(&mut self, state_client: State) {
        let state = match state_client {
            State::ChokeInterested => format!("{} - ", CHOKE) + INTERESTED,
            State::UnchokeInterested => format!("{} - ", UNCHOKE) + INTERESTED,
            State::ChokeNotInterested => format!("{} - ", CHOKE) + NOT_INTERESTED,
            State::UnchokeNotInterested => format!("{} - ", UNCHOKE) + NOT_INTERESTED,
        };
        let state_client_label = self.labels.get(STATE_CLIENT_ID);
        match state_client_label {
            Some(label) => {
                let mut new_state_client = String::from(STATE_CLIENT_LABEL);

                new_state_client.push_str(&state);
                label.set_label(&new_state_client)
            }
            None => (),
        }
    }
}

pub struct MainWindow {
    box_main: gtk::Box,
    stacks_torrents: gtk::Stack,
    stacks_peers: gtk::Stack,
    info_box_torrents: HashMap<String, InfoBox>,
    info_box_peers: HashMap<String, InfoBox>,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MainWindow {
    pub fn new() -> Self {
        let (box_main, stack_main) = create_main_box();
        let (box_stacks_torrent, stacks_torrents) = create_box_for_stacks();
        let (box_stacks_peers, stacks_peers) = create_box_for_stacks();
        let info_box_torrents = HashMap::new();
        let info_box_peers = HashMap::new();

        stack_main.add_titled(
            &box_stacks_torrent,
            Some(TAB_INFO_TORRENT),
            TAB_INFO_TORRENT,
        );
        stack_main.add_titled(&box_stacks_peers, Some(TAB_INFO_PEER), TAB_INFO_PEER);

        MainWindow {
            box_main,
            stacks_torrents,
            stacks_peers,
            info_box_torrents,
            info_box_peers,
        }
    }

    pub fn get_box_main(&self) -> gtk::Box {
        self.box_main.clone()
    }

    pub fn add_torrent(&mut self, torrent_name: String) {
        let mut info_box = InfoBox::new();
        info_box.init_labels_torrent();

        self.stacks_torrents.add_titled(
            info_box.get_box(),
            Some(torrent_name.as_str()),
            torrent_name.as_str(),
        );

        self.info_box_torrents.insert(torrent_name, info_box);
    }

    pub fn add_peer(&mut self, peer_name: String) {
        if let hash_map::Entry::Vacant(_) = self.info_box_peers.entry(peer_name.clone()) {
            let mut info_box = InfoBox::new();
            info_box.init_labels_peers();

            self.stacks_peers.add_titled(
                info_box.get_box(),
                Some(peer_name.as_str()),
                peer_name.as_str(),
            );
            self.info_box_peers.insert(peer_name, info_box);
        }
    }

    pub fn remove_peer(&mut self, peer_name: String) {
        if let Some(info_box) = self.info_box_peers.get(&peer_name) {
            let box_peer = info_box.get_box();
            self.stacks_peers.remove(box_peer);
        }
    }

    pub fn change_tracker(&mut self, torrent: String, tracker: String) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_tracker(tracker);
            }
            None => (),
        }
    }

    pub fn change_info_hash(&mut self, torrent: String, info_hash: String) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_info_hash(info_hash);
            }
            None => (),
        }
    }

    pub fn change_total_size(&mut self, torrent: String, total_size: u64) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_total_size(total_size);
            }
            None => (),
        }
    }

    pub fn change_cant_pieces(&mut self, torrent: String, cant_pieces: u64) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_cant_pieces(cant_pieces);
            }
            None => (),
        }
    }

    pub fn change_peers_leechers(&mut self, torrent: String, seeders: u64, leechers: u64) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_peers_leechers(seeders, leechers);
            }
            None => (),
        }
    }

    pub fn change_single_multiple(&mut self, torrent: String, type_torrent: TorrentFileType) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_single_multiple(type_torrent);
            }
            None => (),
        }
    }

    pub fn change_pieces_downloaded(&mut self, torrent: String, pieces_downloaded: u64) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_pieces_downloaded(pieces_downloaded);
            }
            None => (),
        }
    }

    pub fn change_active_connections(&mut self, torrent: String, type_of_change: TypeOfChange) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_active_connections(type_of_change);
            }
            None => (),
        }
    }

    pub fn change_progress_bar(&mut self, torrent: String, progress: f64) {
        let info_box = self.info_box_torrents.get_mut(&torrent);
        match info_box {
            Some(info_torrent) => {
                info_torrent.change_progress_bar(progress);
            }
            None => (),
        }
    }

    pub fn change_peer_id(&mut self, peer: String, peer_id: String) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_peer_id(peer_id);
            }
            None => (),
        }
    }

    pub fn change_ip(&mut self, peer: String, ip: String) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_ip(ip);
            }
            None => (),
        }
    }

    pub fn change_port(&mut self, peer: String, port: u64) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_port(port);
            }
            None => (),
        }
    }

    pub fn change_download(&mut self, peer: String, download: f64) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_download(download);
            }
            None => (),
        }
    }

    pub fn change_upload(&mut self, peer: String, upload: f64) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_upload(upload);
            }
            None => (),
        }
    }

    pub fn change_state_peer(&mut self, peer: String, state_peer: State) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_state_peer(state_peer);
            }
            None => (),
        }
    }

    pub fn change_state_client(&mut self, peer: String, state_client: State) {
        let info_box = self.info_box_peers.get_mut(&peer);
        match info_box {
            Some(info_peer) => {
                info_peer.change_state_client(state_client);
            }
            None => (),
        }
    }
}
