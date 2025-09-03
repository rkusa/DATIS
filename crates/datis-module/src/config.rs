use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

use datis_core::config::Config;

pub fn read_config(write_dir: &Path) -> Result<Config> {
    let path = write_dir.to_path_buf().join("Config").join("DATIS.json");
    match File::open(&path) {
        Ok(f) => serde_json::from_reader(f).map_err(Error::other),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let new_config = Config::default();
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)?;
            serde_json::to_writer_pretty(f, &new_config).map_err(Error::other)?;
            Ok(new_config)
        }
        Err(err) => Err(err),
    }
}
