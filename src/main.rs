#![allow(unused)]

mod types;

use std::env;

use types::{Blob, FileService, Result};

fn main() {
    FileService::init_blip("/home/toaster/code/p/rust/blip");
    add_file(vec![".gitignore"]);
}

fn add_file(files: Vec<&str>) -> Result<()> {
    let file_service = FileService::new()?;
    let curr_dir = env::current_dir()?;
    let mut index = file_service.read_index()?;

    for file in files {
        let full_path = curr_dir.join(file);
        let blob = Blob::new(&full_path)?;
        file_service.write_blob(&blob);
        let relative_path = full_path
            .strip_prefix(&file_service.root_dir)
            .expect("Error: Invalid File")
            .to_str()
            .expect("Error: Invalid File");
        index.update(&relative_path, &blob.hash());
    }

    file_service
        .write_index(&index)
        .expect("Failed to write to index");
    Ok(())
}
