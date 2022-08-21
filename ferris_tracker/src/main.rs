use std::env;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let new_path = Path::new("./ferris_tracker");
    env::set_current_dir(&new_path)?;
    ferris_tracker::run()
}
