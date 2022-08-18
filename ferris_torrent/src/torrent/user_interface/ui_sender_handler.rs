use core::fmt;
use std::{error::Error, net::SocketAddr, time::Duration};

use super::constants::{MessageUI, State, TorrentFileType, TypeOfChange};
use crate::torrent::data::{
    peer_data_for_communication::PeerDataForP2PCommunication, torrent_file_data::TorrentFileData,
    torrent_status::TorrentStatus, tracker_response_data::TrackerResponseData,
};
use gtk::glib::Sender;

#[derive(PartialEq, Eq, Debug)]
pub enum UiError {
    AddingTorrent(String),
    UpdatingTorrentInformation(String),
    AddingPeer(String),
    RemovingPeer(String),
    UpdatingPeersState(String),
    UpdatingUpload(String),
    UpdatingDownload(String),
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for UiError {}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|a| format!("{:02x}", a)).collect()
}

pub fn add_torrent(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
) -> Result<(), UiError> {
    ui_sender
        .send(MessageUI::AddTorrent {
            torrent_name: torrent_file.get_torrent_representative_name(),
        })
        .map_err(|err| UiError::AddingTorrent(format!("{}", err)))
}

pub fn update_torrent_status(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    torrent_status: &TorrentStatus,
) -> Result<(), UiError> {
    ui_sender
        .send(MessageUI::UpdatePiecesDownloaded {
            torrent_name: torrent_file.get_torrent_representative_name(),
            pieces_downloaded: torrent_status.get_amount_of_downloaded_pieces(),
        })
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;

    let porcentage_downloaded = torrent_status
        .get_porcentage_downloaded()
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;

    ui_sender
        .send(MessageUI::UpdatePorcentageDownloaded {
            torrent_name: torrent_file.get_torrent_representative_name(),
            porcentage_downloaded,
        })
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;

    Ok(())
}

pub fn update_torrent_information(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    tracker_response: &TrackerResponseData,
    torrent_status: &TorrentStatus,
) -> Result<(), UiError> {
    let cant_pieces = torrent_file
        .get_total_amount_pieces()
        .try_into()
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;

    let total_size = u32::try_from((torrent_file.get_total_length()) / 1000)
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;
    let total_size = f64::from(total_size);

    ui_sender
        .send(MessageUI::UpdateTorrentData {
            torrent_name: torrent_file.get_torrent_representative_name(),
            tracker_url: torrent_file.get_tracker_main(),
            info_hash: to_hex(&torrent_file.get_info_hash()),
            total_size,
            cant_pieces,
            seeders: tracker_response.get_total_amount_seeders(),
            leechers: tracker_response.get_total_amount_leechers(),
            type_torrent: TorrentFileType::SingleFile,
        })
        .map_err(|err| UiError::UpdatingTorrentInformation(format!("{}", err)))?;

    update_torrent_status(ui_sender, torrent_file, torrent_status)?;

    Ok(())
}

pub fn update_peers_state(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    external_peer: &PeerDataForP2PCommunication,
) -> Result<(), UiError> {
    let torrent_name = torrent_file.get_torrent_representative_name();
    let peer_name = external_peer
        .get_peer_name()
        .map_err(|err| UiError::UpdatingPeersState(format!("{}", err)))?;
    let peer_name = format!("[{}] Peer: {} ", torrent_name, peer_name);

    let state_client = if external_peer.peer_choking {
        if external_peer.am_interested {
            State::ChokeInterested
        } else {
            State::ChokeNotInterested
        }
    } else if external_peer.am_interested {
        State::UnchokeInterested
    } else {
        State::UnchokeNotInterested
    };

    ui_sender
        .send(MessageUI::UpdateClientState {
            peer_name: peer_name.clone(),
            state_client,
        })
        .map_err(|err| UiError::UpdatingPeersState(format!("{}", err)))?;

    let state_peer = if external_peer.am_choking {
        if external_peer.peer_interested {
            State::ChokeInterested
        } else {
            State::ChokeNotInterested
        }
    } else if external_peer.am_interested {
        State::UnchokeInterested
    } else {
        State::UnchokeNotInterested
    };

    ui_sender
        .send(MessageUI::UpdatePeerState {
            peer_name,
            state_peer,
        })
        .map_err(|err| UiError::UpdatingPeersState(format!("{}", err)))?;
    Ok(())
}

pub fn add_external_peer(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    external_peer: &PeerDataForP2PCommunication,
    external_peer_addr: &SocketAddr,
) -> Result<(), UiError> {
    let torrent_name = torrent_file.get_torrent_representative_name();
    let peer_name = external_peer
        .get_peer_name()
        .map_err(|err| UiError::AddingPeer(format!("{}", err)))?;
    let peer_name = format!("[{}] Peer: {} ", torrent_name, peer_name);

    ui_sender
        .send(MessageUI::UpdateActiveConnections {
            torrent_name,
            type_of_change: TypeOfChange::Addition,
        })
        .map_err(|err| UiError::AddingPeer(format!("{}", err)))?;

    ui_sender
        .send(MessageUI::AddPeer {
            peer_name: peer_name.clone(),
        })
        .map_err(|err| UiError::AddingPeer(format!("{}", err)))?;

    ui_sender
        .send(MessageUI::UpdatePeerData {
            peer_name,
            peer_id: to_hex(&external_peer.get_peer_id()),
            ip: external_peer_addr.ip().to_string(),
            port: external_peer_addr.port().into(),
        })
        .map_err(|err| UiError::AddingPeer(format!("{}", err)))?;

    update_peers_state(ui_sender, torrent_file, external_peer)?;

    update_download_data(
        ui_sender,
        torrent_file,
        external_peer,
        0,
        Duration::from_secs(1),
    )?;
    update_upload_data(
        ui_sender,
        torrent_file,
        external_peer,
        0,
        Duration::from_secs(1),
    )?;

    Ok(())
}

pub fn remove_external_peer(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    external_peer: &PeerDataForP2PCommunication,
) -> Result<(), UiError> {
    let torrent_name = torrent_file.get_torrent_representative_name();
    let peer_name = external_peer
        .get_peer_name()
        .map_err(|err| UiError::RemovingPeer(format!("{}", err)))?;
    let peer_name = format!("[{}] Peer: {} ", torrent_name, peer_name);

    ui_sender
        .send(MessageUI::UpdateActiveConnections {
            torrent_name,
            type_of_change: TypeOfChange::Substraction,
        })
        .map_err(|err| UiError::RemovingPeer(format!("{}", err)))?;

    ui_sender
        .send(MessageUI::RemovePeer { peer_name })
        .map_err(|err| UiError::RemovingPeer(format!("{}", err)))?;

    Ok(())
}

pub fn update_upload_data(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    external_peer: &PeerDataForP2PCommunication,
    upload_bytes: u64,
    upload_duration: Duration,
) -> Result<(), UiError> {
    let torrent_name = torrent_file.get_torrent_representative_name();
    let peer_name = external_peer
        .get_peer_name()
        .map_err(|err| UiError::UpdatingUpload(format!("{}", err)))?;
    let peer_name = format!("[{}] Peer: {} ", torrent_name, peer_name);

    let upload_bytes = match u32::try_from(upload_bytes) {
        Ok(bytes) => bytes,
        Err(error) => return Err(UiError::UpdatingUpload(format!("{}", error))),
    };
    let upload_bytes = f64::from(upload_bytes);

    let upload = upload_bytes / upload_duration.as_secs_f64();

    ui_sender
        .send(MessageUI::UpdateUpload { peer_name, upload })
        .map_err(|err| UiError::UpdatingUpload(format!("{}", err)))?;

    Ok(())
}

pub fn update_download_data(
    ui_sender: &Sender<MessageUI>,
    torrent_file: &TorrentFileData,
    external_peer: &PeerDataForP2PCommunication,
    download_bytes: u64,
    download_duration: Duration,
) -> Result<(), UiError> {
    let torrent_name = torrent_file.get_torrent_representative_name();
    let peer_name = external_peer
        .get_peer_name()
        .map_err(|err| UiError::UpdatingDownload(format!("{}", err)))?;
    let peer_name = format!("[{}] Peer: {} ", torrent_name, peer_name);

    let download_bytes = match u32::try_from(download_bytes) {
        Ok(bytes) => bytes,
        Err(error) => return Err(UiError::UpdatingDownload(format!("{}", error))),
    };
    let download_bytes = f64::from(download_bytes);

    let download = download_bytes / download_duration.as_secs_f64();

    ui_sender
        .send(MessageUI::UpdateDownload {
            peer_name,
            download,
        })
        .map_err(|err| UiError::UpdatingDownload(format!("{}", err)))?;

    Ok(())
}
