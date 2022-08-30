pub const ID_APP: &str = "org.gtk.torrent";
pub const NAME_APP: &str = "Ferris Torrent";

pub const TAB_INFO_TORRENT: &str = "Informacion general";
pub const TAB_INFO_PEER: &str = "Estadisticas de descargas";

pub const CSS_STACK_MAIN: &str = "stack_main";
pub const CSS_STACK_INFO: &str = "stack_info";

pub const ZERO: u64 = 0;
pub const ONE: f64 = 1.0;
pub const FIVE: i32 = 5;
pub const MIN_SCROLL_WIDTH: i32 = 300;

pub const INCOGNITO: &str = "?";

pub const IMAGE_FERRIS: &str = "ferris_torrent/images/ferris.png";
pub const IMAGE_FERRIS2: &str = "ferris_torrent/images/ferris2.png";
pub const IMAGE_FERRIS3: &str = "ferris_torrent/images/ferris_finish.png";

pub const TRACKER_ID: &str = "tracker";
pub const INFO_HASH_ID: &str = "info_hash";
pub const TOTAL_SIZE_ID: &str = "total_size";
pub const CANT_PIECES_ID: &str = "cant_pieces";
pub const PEERS_LEECHERS_ID: &str = "peers_leechers";
pub const SINGLE_MULTIPLE_ID: &str = "single_multiple";
pub const PIECES_DOWNLOADED_ID: &str = "pieces_downloaded";
pub const ACTIVE_CONNECTIONS_ID: &str = "active_connections";

pub const PEER_ID_ID: &str = "peer_id";
pub const IP_ID: &str = "ip";
pub const PORT_ID: &str = "port";
pub const DOWNLOAD_ID: &str = "download";
pub const UPLOAD_ID: &str = "upload";
pub const STATE_PEER_ID: &str = "state_peer";
pub const STATE_CLIENT_ID: &str = "state_client";

pub const TRACKER_LABEL: &str = "Tracker = ";
pub const INFO_HASH_LABEL: &str = "Info Hash = ";
pub const TOTAL_SIZE_LABEL: &str = "Total size = ";
pub const CANT_PIECES_LABEL: &str = "Cant. pieces = ";
pub const SEEDERS_LABEL: &str = "Seeders = ";
pub const LEECHERS_LABEL: &str = ", Leechers = ";
pub const PIECES_DOWNLOADED_LABEL: &str = "Pieces downloaded = ";
pub const ACTIVE_CONNECTIONS_LABEL: &str = "Active connections = ";

pub const PEER_ID_LABEL: &str = "Peer ID = ";
pub const IP_LABEL: &str = "IP = ";
pub const PORT_LABEL: &str = "Port = ";
pub const DOWNLOAD_LABEL: &str = "Download speed = ";
pub const UPLOAD_LABEL: &str = "Upload speed = ";
pub const STATE_PEER_LABEL: &str = "Peer state = ";
pub const STATE_CLIENT_LABEL: &str = "Client state = ";

pub const SINGLE_FILE: &str = "Single File";
pub const MULTIPLE_FILE: &str = "Multiple File";

pub const CHOKE: &str = "Choke";
pub const UNCHOKE: &str = "Unchoked";
pub const INTERESTED: &str = "Interested";
pub const NOT_INTERESTED: &str = "Not interested";

pub const KILOBYTES: &str = " [kiB]";
pub const KILOBYTESPERSEC: &str = " [kiB/sec]";

pub enum TypeOfChange {
    Addition,
    Substraction,
}

pub enum TorrentFileType {
    SingleFile,
    MultipleFile,
}
#[allow(dead_code)]
pub enum State {
    ChokeInterested,
    ChokeNotInterested,
    UnchokeInterested,
    UnchokeNotInterested,
}

pub enum MessageUI {
    //-- Torrent
    AddTorrent {
        torrent_name: String,
    },
    UpdateTorrentData {
        torrent_name: String,
        tracker_url: String,
        info_hash: String,
        total_size: f64,
        cant_pieces: u64,
        seeders: u64,
        leechers: u64,
        type_torrent: TorrentFileType,
    },
    //-- Status
    UpdatePiecesDownloaded {
        torrent_name: String,
        pieces_downloaded: u64,
    },
    UpdatePorcentageDownloaded {
        torrent_name: String,
        porcentage_downloaded: f64,
    },
    //-- Internet
    UpdateDownload {
        peer_name: String,
        download: f64,
    },
    UpdateUpload {
        peer_name: String,
        upload: f64,
    },
    //-- Peers
    UpdateActiveConnections {
        torrent_name: String,
        type_of_change: TypeOfChange,
    },
    AddPeer {
        peer_name: String,
    },
    RemovePeer {
        peer_name: String,
    },
    UpdateClientState {
        peer_name: String,
        state_client: State,
    },
    UpdatePeerData {
        peer_name: String,
        peer_id: String,
        ip: String,
        port: u64,
    },
    UpdatePeerState {
        peer_name: String,
        state_peer: State,
    },
    //-- Other
    Shutdown,
}
