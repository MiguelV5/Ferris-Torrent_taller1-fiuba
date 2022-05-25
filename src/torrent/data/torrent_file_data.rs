#[derive(PartialEq, Debug, Clone)]
pub struct TorrentFileData {
    pub piece_lenght: u32,
    pub total_amount_pieces: usize,
}
