use std::cell::RefCell;

use super::constants::*;
use super::main_window::MainWindow;

use gtk::glib::{self, Receiver, Sender};

use gtk::gdk::Display;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, CssProvider, StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION,
};

pub fn build_app() -> (Application, Sender<MessageUI>) {
    let app = Application::builder().application_id(ID_APP).build();

    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let receiver: RefCell<Option<Receiver<MessageUI>>> = RefCell::new(Some(receiver));

    app.connect_startup(move |app| {
        let provider = CssProvider::new();
        provider.load_from_data(include_bytes!("style.css"));
        let display = Display::default().expect("Error al conectar a Display.");
        StyleContext::add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let receiver = receiver.take().expect("Error en el reciever");
        build_ui(app, receiver);
    });

    (app, sender)
}

fn build_ui(app: &Application, receiver: Receiver<MessageUI>) {
    let mut builder_main = MainWindow::new();
    let boxed_main = builder_main.get_box_main();

    let window = ApplicationWindow::builder()
        .application(app)
        .title(NAME_APP)
        .default_width(850)
        .default_height(550)
        .resizable(false)
        .child(&boxed_main)
        .build();

    let window_clone = window.clone();

    receiver.attach(None, move |msg| {
        match msg {
            //Mensajes de torrents
            MessageUI::AddTorrent { torrent_name } => builder_main.add_torrent(torrent_name),
            MessageUI::UpdateTorrentData {
                torrent_name,
                tracker_url,
                info_hash,
                total_size,
                cant_pieces,
                seeders: peers,
                leechers,
                type_torrent,
            } => {
                builder_main.change_tracker(torrent_name.clone(), tracker_url);
                builder_main.change_info_hash(torrent_name.clone(), info_hash);
                builder_main.change_total_size(torrent_name.clone(), total_size);
                builder_main.change_cant_pieces(torrent_name.clone(), cant_pieces);
                builder_main.change_peers_leechers(torrent_name.clone(), peers, leechers);
                builder_main.change_single_multiple(torrent_name, type_torrent);
            }
            MessageUI::UpdatePiecesDownloaded {
                torrent_name,
                pieces_downloaded,
            } => builder_main.change_pieces_downloaded(torrent_name, pieces_downloaded),
            MessageUI::UpdateActiveConnections {
                torrent_name,
                type_of_change,
            } => builder_main.change_active_connections(torrent_name, type_of_change),
            MessageUI::UpdatePorcentageDownloaded {
                torrent_name,
                porcentage_downloaded,
            } => builder_main.change_progress_bar(torrent_name, porcentage_downloaded),

            //Mensajes de peers
            MessageUI::AddPeer { peer_name } => builder_main.add_peer(peer_name),
            MessageUI::RemovePeer { peer_name } => builder_main.remove_peer(peer_name),
            MessageUI::UpdatePeerData {
                peer_name,
                peer_id,
                ip,
                port,
            } => {
                builder_main.change_peer_id(peer_name.clone(), peer_id);
                builder_main.change_ip(peer_name.clone(), ip);
                builder_main.change_port(peer_name, port);
            }
            MessageUI::UpdateDownload {
                peer_name,
                download,
            } => builder_main.change_download(peer_name, download),
            MessageUI::UpdateUpload { peer_name, upload } => {
                builder_main.change_upload(peer_name, upload)
            }
            MessageUI::UpdatePeerState {
                peer_name,
                state_peer,
            } => builder_main.change_state_peer(peer_name, state_peer),
            MessageUI::UpdateClientState {
                peer_name,
                state_client,
            } => builder_main.change_state_client(peer_name, state_client),

            MessageUI::Shutdown => window_clone.close(),
        }
        glib::Continue(true)
    });

    app.connect_activate(move |_| {
        window.show();
    });
}
