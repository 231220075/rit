use std::path::{PathBuf,Path};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter, Read, BufReader, BufRead};
use byteorder::{ReadBytesExt, BigEndian};
#[derive(Debug)]
pub struct IndexEntry {
    pub mode: u32,          
    pub hash: String,       
    pub name: String,      
}

impl IndexEntry {
    
        pub fn new(mode: u32, hash: String, name: String) -> Self {
            match mode {
                0o100644 | 0o100755 | 0o120000 | 0o040000 => (),
                _ => panic!("Invalid file mode: {:o}", mode),
            }
            IndexEntry { mode, hash, name }
        }
    
}

pub struct Index {
    pub entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Index { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.push(entry);
    }

    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);
    
        for entry in &self.entries {
            writer.write_all(&entry.mode.to_be_bytes())?; 
    
            let hash_bytes = hex::decode(&entry.hash).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid hash format")
            })?;
            writer.write_all(&hash_bytes)?; 
    
            writer.write_all(entry.name.as_bytes())?; 
            writer.write_all(&[0])?; 
        }
        Ok(())
    }

    pub fn read_from_file(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut index = Index::new();
    
        while let Ok(mode) = reader.read_u32::<BigEndian>() {
            let mut hash = [0u8; 20];
            reader.read_exact(&mut hash)?;
    
            let mut name = Vec::new();
            reader.read_until(0, &mut name)?;
            if name.is_empty() || name.last() != Some(&0) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid name format in index file",
                ));
            }
            name.pop(); 
    
            let name_str = String::from_utf8(name).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8 in name")
            })?;
    
            index.add_entry(IndexEntry::new(
                mode,
                hex::encode(hash),
                name_str,
            ));
        }
        Ok(index)
    }
}