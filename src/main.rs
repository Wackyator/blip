#![allow(unused)]

mod types;

use std::{env};

use types::{Blob, FileService, Result};

fn main() {
    FileService::init_blip("/home/toaster/code/p/rust/blip");
    add_file(vec![".gitignore"]); 
}

fn add_file(files: Vec<&str>) -> Result<()> {
    let file_service = FileService::new()?;
    println!("{:?}", file_service);
    let curr_dir = env::current_dir()?;
    let mut index = file_service.read_index()?;
    println!("{:?}", index);

    for file in files {
        let full_path = curr_dir.join(file);
        println!("{:?}", full_path);
        let blob = Blob::new(&full_path)?;
        println!("{:#?}", blob);
        file_service.write_blob(&blob);
        let relative_path = full_path
            .strip_prefix(&file_service.root_dir)
            .expect("Error: Invalid File")
            .to_str()
            .expect("Error: Invalid File");
        println!("{:?}", relative_path);
        index.update(&relative_path, &blob.hash());
    }

    println!("{:?}", index);
    file_service.write_index(&index).expect("Failed to write to index");
    Ok(())
}
