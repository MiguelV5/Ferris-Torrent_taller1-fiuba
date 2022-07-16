use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    ferris_torrent::run()
}
