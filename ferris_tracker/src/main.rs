use log::info;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    info!("tracker init");
    Ok(())
}
