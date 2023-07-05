use std::{
    collections::BTreeMap,
    env, fmt,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

use crypto::{digest::Digest, sha1::Sha1};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    NoDirectory,
    InvalidIndex,
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
}

#[derive(Debug)]
pub struct Index {
    pub path: PathBuf,
    pub hashtree: BTreeMap<String, String>,
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => e.fmt(fmt),
            Self::NoDirectory => fmt.write_str("No Directory Found"),
            Self::InvalidIndex => fmt.write_str("Index is Corrupt"),
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

        Ok(FileService {
            root_dir,
            blip_dir,
            object_dir,
            index,
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

    pub fn write_index(&self, index: &Index) -> Result<()> {
        let mut file = File::create(self.index.clone())?;
        for (hash, path) in index.hashtree().iter() {
            println!("writing: {hash}, {path}");
            file.write_all(format!("{} {}", hash, path).as_bytes())?;
            // writeln!(&mut file, "{} {}", hash, path);
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
}
