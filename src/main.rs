#![allow(unused)]

mod types;

use std::{env, process::exit};

use types::{Blob, Commit, Error, FileService, Result};

fn main() {
    // FileService::init_blip("/home/toaster/code/p/rust/blip").unwrap();
    // add_file(vec![".gitignore"]).unwrap();
    commit("commit text").unwrap();
    // let fs = FileService::new().unwrap();
    // println!("{:?}", fs.get_head_ref());
}

fn commit(msg: &str) -> Result<()> {
    let file_service = FileService::new()?;
    let head_ref = file_service.get_head_ref()?;
    let parent_hash = FileService::get_hash_from_ref(&head_ref);
    let mut index = file_service.read_index()?;

    let parent = match parent_hash {
        Some(hash) => Some(file_service.read_commit(&hash)?),
        None => None,
    };

    let mut commit = Commit::new(parent.as_ref());
    parent.map(|p| p.print());
    commit.add_from_index(&index);
    commit.print();

    file_service.write_commit(&mut commit)?;
    index.clear()?;
    println!("{msg}");
    Ok(())
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
