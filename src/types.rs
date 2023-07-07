use std::{
    collections::BTreeMap,
    env, fmt,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::prelude::FileExt,
    path::{Path, PathBuf},
};

use crypto::{digest::Digest, sha1::Sha1};
use regex::Regex;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NoDirectory,
    InvalidIndex,
    InvalidObjectStore,
    EmptyCommit,
}

#[derive(Debug)]
pub enum ObjectStore {
    Blob(Blob),
    Tree(Tree),
}

#[derive(Debug)]
pub struct Blob {
    hash: String,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Tree {
    tree_type: String,
    name: String,
    hash: String,
    children: Vec<ObjectStore>,
}

#[derive(Debug)]
pub struct FileService {
    pub root_dir: PathBuf,
    pub blip_dir: PathBuf,
    pub object_dir: PathBuf,
    pub index: PathBuf,
    pub head: PathBuf,
}

#[derive(Debug)]
pub struct Index {
    pub path: PathBuf,
    pub hashtree: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct Commit {
    hash: Option<String>,
    data: Option<Vec<u8>>,
    parent: Option<String>,
    files: BTreeMap<String, String>,
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => e.fmt(fmt),
            Self::NoDirectory => fmt.write_str("No Directory Found"),
            Self::InvalidIndex => fmt.write_str("Index is Corrupt"),
            Self::InvalidObjectStore => fmt.write_str("Blip Repository is Corrupt"),
            Self::EmptyCommit => fmt.write_str("No Files Staged for Commit"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl Blob {
    pub fn new(path: &PathBuf) -> Result<Blob> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();

        file.read_to_end(&mut data)?;

        let mut hash = Sha1::new();
        hash.input(&data);

        Ok(Blob {
            hash: hash.result_str(),
            data,
        })
    }
}

impl Blob {
    pub fn hash(&self) -> &String {
        return &self.hash;
    }

    pub fn data(&self) -> &Vec<u8> {
        return &self.data;
    }
}

impl FileService {
    pub fn new() -> Result<FileService> {
        let root_dir = FileService::find_root()?;
        let blip_dir = root_dir.join(".blip");
        let object_dir = blip_dir.join("objects");
        let index = blip_dir.join("index");
        let head = blip_dir.join("HEAD");

        Ok(FileService {
            root_dir,
            blip_dir,
            object_dir,
            index,
            head,
        })
    }

    pub fn init_blip(path: &str) -> Result<()> {
        let path: PathBuf = [path, ".blip"].iter().collect();

        fs::create_dir_all(path.join("objects"))?;
        fs::create_dir_all(path.join("refs").join("heads"))?;

        File::create(path.join("index"))?;
        let mut head = File::create(path.join("HEAD"))?;
        head.write_all("ref: refs/heads/master".as_bytes());

        Ok(())
    }

    fn find_root() -> Result<PathBuf> {
        let mut current_dir = env::current_dir()?;
        loop {
            if FileService::is_blip(&current_dir) {
                return Ok(current_dir);
            }
            if !current_dir.pop() {
                return Err(Error::NoDirectory);
            }
        }
    }

    fn is_blip<P>(path: P) -> bool
    where
        P: Sized + AsRef<Path>,
    {
        path.as_ref().join(".blip").exists()
    }
}

impl FileService {
    pub fn get_head_ref(&self) -> Result<PathBuf> {
        let mut head_file = File::open(self.head.clone())?;
        let mut ref_path = String::new();
        head_file.read_to_string(&mut ref_path)?;
        let ref_path = ref_path.split_off(5);

        Ok(self.blip_dir.join(ref_path))
    }

    pub fn get_hash_from_ref(ref_path: &PathBuf) -> Option<String> {
        match File::open(ref_path) {
            Ok(mut f) => {
                let mut hash = String::new();
                f.read_to_string(&mut hash)
                    .expect("Error: Ref File is Corrupt");
                return Some(hash);
            }
            Err(_) => None,
        }
    }

    pub fn read_commit(&self, hash: &str) -> Result<Commit> {
        Commit::from(hash, &self.read_object(hash)?)
    }

    fn read_object(&self, hash: &str) -> Result<String> {
        let mut data = String::new();
        let mut object_file = File::open(self.object_dir.join(hash))?;
        object_file.read_to_string(&mut data)?;

        Ok(data)
    }

    pub fn read_index(&self) -> Result<Index> {
        let mut index_data = BTreeMap::new();

        let file = BufReader::new(File::open(&self.index)?);
        for line in file.lines() {
            let line = line?;
            let blob: Vec<_> = line.split(' ').collect();
            if blob.len() != 2 {
                return Err(Error::InvalidIndex);
            }
            index_data.insert(blob[0].to_string(), blob[1].to_string());
        }

        Ok(Index::new(self.index.clone(), index_data))
    }

    pub(crate) fn write_commit(&self, commit: &mut Commit) -> Result<()> {
        commit.update();

        match commit {
            &mut Commit {
                hash: Some(ref hash),
                data: Some(ref data),
                ..
            } => {
                self.write_obj(hash, data)?;
                let mut head_file = File::create(self.get_head_ref()?)?;
                head_file.write_all(hash.as_bytes())?;
            }
            _ => {
                return Err(Error::EmptyCommit);
            }
        }

        Ok(())
    }

    pub fn write_index(&self, index: &Index) -> Result<()> {
        let mut file = File::create(self.index.clone())?;
        for (hash, path) in index.hashtree().iter() {
            writeln!(&mut file, "{} {}", hash, path);
        }
        Ok(())
    }

    pub fn write_blob(&self, blob: &Blob) -> Result<()> {
        self.write_obj(blob.hash(), blob.data())
    }

    fn write_obj(&self, hash: &str, data: &Vec<u8>) -> Result<()> {
        let mut blob = File::create(self.object_dir.join(hash))?;
        blob.write_all(data)?;

        Ok(())
    }
}

impl Index {
    fn new(path: PathBuf, hashtree: BTreeMap<String, String>) -> Self {
        Index { path, hashtree }
    }
}

impl Index {
    fn hashtree(&self) -> &BTreeMap<String, String> {
        &self.hashtree
    }

    pub fn update(&mut self, path: &str, hash: &str) {
        self.hashtree.insert(path.to_string(), hash.to_string());
    }

    pub(crate) fn clear(&mut self) -> Result<()> {
        self.hashtree = BTreeMap::new();
        self.write()?;
        Ok(())
    }

    fn write(&self) -> Result<()> {
        let mut index = File::create(&self.path)?;
        for (hash, path) in self.hashtree.iter() {
            writeln!(&mut index, "{hash} {path}");
        }
        Ok(())
    }
}

impl Commit {
    pub fn new(parent: Option<&Commit>) -> Commit {
        let mut commit = Commit {
            hash: None,
            data: None,
            parent: match parent {
                Some(&Commit {
                    hash: Some(ref hash),
                    ..
                }) => Some(hash.to_string()),
                _ => None,
            },
            files: BTreeMap::new(),
        };

        for (hash, path) in parent.iter().flat_map(|p| p.files.iter()) {
            commit.files.insert(hash.to_string(), path.to_string());
        }

        commit
    }

    pub fn from(hash: &str, input: &str) -> Result<Commit> {
        let mut commit = Commit::new(None);
        commit.hash = Some(hash.to_string());

        let parent = Regex::new(r"parent ([0-9a-f]{40})").unwrap();
        let blob = Regex::new(r"blob ([0-9a-f]{40}) (.*)").unwrap();

        for line in input.lines() {
            if let Some(caps) = parent.captures(line) {
                // this syntax is ugly looking but is definitely better than panicing imo
                // alternate way to do this would be
                // commit.parent = Some(caps.get().unwrap().as_str().into());
                // or
                // commit.parent = Some(caps.get(1).ok_or_else(|| Error::InvalidObjectStore)?.as_str().into());
                // if-let-else can be used together to all this at once but looks cluttered
                let Some(hash) = caps.get(1) else {
                    return Err(Error::InvalidObjectStore);
                };
                commit.parent = Some(hash.as_str().into());
            }

            if let Some(caps) = blob.captures(line) {
                let Some(hash) = caps.get(1) else {
                    return Err(Error::InvalidObjectStore);
                };
                let Some(ref path) = caps.get(3) else {
                    return Err(Error::InvalidObjectStore);
                };

                commit
                    .files
                    .insert(hash.as_str().to_string(), path.as_str().to_string());
            }
        }

        Ok(commit)
    }
}

impl Commit {
    pub(crate) fn print(&self) {
        if let Some(ref parent) = self.parent {
            println!("parent {parent}");
        }

        for (hash, path) in self.files.iter() {
            println!("blob {hash} {path}");
        }
    }

    pub(crate) fn add_from_index(&mut self, index: &Index) {
        for (hash, path) in index.hashtree().iter() {
            self.files.insert(hash.to_string(), path.to_string());
        }
    }

    pub(crate) fn update(&mut self) {
        let mut data: Vec<u8> = Vec::new();

        if let Some(ref parent) = self.parent {
            writeln!(&mut data, "parent {parent}");
        }

        for (hash, path) in self.files.iter() {
            writeln!(&mut data, "blob {hash}, {path}");
        }

        let mut hash = Sha1::new();
        hash.input(&data);
        self.hash = Some(hash.result_str());
        self.data = Some(data);
    }
}
