use std::{
    fmt,
    result,
    ops::Deref,
    convert::{
        TryFrom,
    },
    error::Error,
    path::PathBuf,
    iter::Iterator,
};

use nom::{
    Parser,
    bytes::complete::{tag, take, take_until},
    number::complete::be_u32,
    character::complete::{alpha1, digit1, space1, u32},
    sequence::{
        terminated,
        preceded,
        separated_pair,
    },
    multi::{
        many0,
        many1,
    },
    combinator::{
        map, map_res, eof, value,
    },
    IResult,
};
use hex::encode;

use crate::utils::{
    objtype::{
        ObjType,
        parse_meta,
    },
    error::{
        GitError,
        Result
    },
};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    Blob     = 0o100644,
    Tree     = 0o040000,
    Commit   = 0o160000,
    Symbolic = 0o120000,
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
            match self {
                FileMode::Blob => "blob",
                FileMode::Tree => "tree",
                FileMode::Commit => "commit",
                FileMode::Symbolic => "symbolic",
            })
    }
}

impl TryFrom<&[u8]> for FileMode {
    type Error = Box<dyn Error>;

    fn try_from(bytes: &[u8]) -> result::Result<Self, Self::Error> {
        let mode = String::from_utf8(bytes.to_vec())?;
        match mode.as_str() {
            "100644" => Ok(FileMode::Blob),
            "40000"  => Ok(FileMode::Tree),
            "160000" => Ok(FileMode::Commit),
            "120000" => Ok(FileMode::Symbolic),
            other    => Err(GitError::invalid_filemode(other.to_string()))
        }
    }
}

#[derive(Clone)]
pub struct TreeEntry {
    pub mode: FileMode, //
    pub hash: String,   // 40 length
    pub path: PathBuf,  // relative to git dir
}

type EntryPrototype<'a> = (&'a[u8], &'a[u8], &'a[u8]);
impl<'a> TryFrom<EntryPrototype<'a>> for TreeEntry {
    type Error = Box<dyn Error>;

    fn try_from(enp: EntryPrototype) -> result::Result<Self, Self::Error> {
        let mode = enp.0.try_into()?;
        let path = PathBuf::from(&String::from_utf8(enp.1.to_vec())?);
        let hash = encode(enp.2);
        Ok(TreeEntry {
            mode,
            hash,
            path,
        })
    }
}

impl TreeEntry {
    fn parse_from_bytes(bytes: &[u8]) -> IResult<&[u8], EntryPrototype> {
        let parse_mode = terminated(take_until(" "), tag(" "));
        let parse_path = terminated(take_until("\0"), tag("\0"));
        let parse_hash = take(20usize);

        (
            parse_mode,
            parse_path,
            parse_hash,
        ).parse(bytes)
    }

    fn into_iter(self) -> impl Iterator<Item = u8> {
        let mode = (self.mode as u32).to_be_bytes().to_vec();
        let hash = self.hash.into_bytes();
        let path = self.path.to_str().unwrap().as_bytes().to_vec();

        mode.into_iter()
            .chain(hash)
            .chain(path)
    }

}


impl TryFrom<&[u8]> for TreeEntry {
    type Error = Box<dyn Error>;

    fn try_from(bytes: &[u8]) -> result::Result<Self, Self::Error> {
        // [mode] space [path] 0x00 [sha-1]

        let (_, (modebytes, hashbytes, pathbytes)) = TreeEntry::parse_from_bytes(bytes)
                                                                                    .map_err(GitError::invalid_entry)?;

        let mode = modebytes.try_into()?;
        let path = PathBuf::from(String::from_utf8(pathbytes.to_vec())?);
        let hash = String::from_utf8(hashbytes.to_vec())?;
        Ok(TreeEntry {
            mode,
            hash,
            path,
        })
    }
}

impl fmt::Display for TreeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06o} {} {}\t{}", self.mode as u32, self.mode, self.hash.clone(), self.path.display())
    }
}

pub struct Tree(pub Vec<TreeEntry>);

impl TryFrom<Vec<u8>> for Tree {
    type Error = Box<dyn Error>;

    fn try_from(bytes: Vec<u8>) -> result::Result<Self, Self::Error> {
        let (entrys, _) = parse_meta(bytes.as_slice()).map_err(GitError::invalid_tree)?;

        let (_, entrys_str) = terminated(many1(TreeEntry::parse_from_bytes), eof)
            .parse(entrys)
            .map_err(GitError::invalid_entry)?;
        let entrys = entrys_str
            .into_iter()
            .map(|prototype| prototype.try_into())
            .collect::<Result<Vec<TreeEntry>>>()?;

        Ok(Tree(entrys))
    }
}

impl From<Tree> for Vec<u8> {
    fn from(val: Tree) -> Vec<u8> {
        val.0
            .into_iter()
            .flat_map(|x|x.into_iter())
            .collect()
    }
}


impl ObjType for Tree {
    const VALUE: &'static str = "tree";
    const MODE: u32 = 0o040000;
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.0.iter().map(|x|x.to_string()).collect::<Vec<_>>().join("\n"))
    }
}
