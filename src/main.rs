#![allow(unused)]

use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

fn main() {
    init_blip("/home/toaster/code/p/rust/blip");
}

fn init_blip(path: &str) -> Result<(), io::Error> {
    let path: PathBuf = [path, ".blip"].iter().collect();

    fs::create_dir_all(path.join("objects"))?;
    fs::create_dir_all(path.join("refs").join("heads"))?;

    let mut head = File::create(path.join("HEAD"))?;
    head.write_all("ref: refs/heads/master".as_bytes());

    Ok(())
}
