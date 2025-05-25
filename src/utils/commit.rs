use std::{
    fmt,
    result,
    convert::TryFrom,
    error::Error,
    path::PathBuf
};

use crate::utils:: {
    objtype::{
        ObjType,
        parse_meta,
    },
    error::{
        GitError,
        Result
    },
};

use nom::{
    Parser,
    bytes::complete::{
        tag, take, take_until, take_while,
    },
    number::complete::be_u32,
    character::complete::{digit1, space1, u32, alpha1},
    sequence::{
        terminated,
        preceded,
        separated_pair,
    },
    branch::alt,
    combinator::{
        map, map_res, eof, value, opt
    },
    IResult,
};


pub struct Commit {
    tree_hash: String,
    parent_hash: Option<String>,
    author: String,
    committer: String,
    message: String,
}

impl Commit {
    fn parse_from_bytes(bytes: &[u8]) -> IResult<& [u8], (&[u8], Option<&[u8]>, &[u8], &[u8], &[u8])> {
        let parse_tree = terminated(preceded(tag("tree "),take_until("\n")), tag("\n"));
        let parse_parent = opt(terminated(preceded(tag("parent "),take_until("\n")), tag("\n")));
        let parse_author = terminated(preceded(tag("author "),take_until("\n")), tag("\n"));
        let parse_committer = terminated(preceded(tag("committer "),take_until("\n")), tag("\n"));
        let parse_messages = preceded(tag("\n"), take_while(|_|true));
        (
            parse_tree,
            parse_parent,
            parse_author,
            parse_committer,
            parse_messages,
        ).parse(bytes)
    }
}

impl ObjType for Commit {
    const VALUE: &'static str = "commit";
    const MODE: u32 = 0o160000;
}

impl TryFrom<Vec<u8>> for Commit {
    type Error = Box<dyn Error>;

    fn try_from(bytes: Vec<u8>) -> result::Result<Self, Self::Error> {
        let ( _,
                (_, (tree_hash, parent_hash, author, committer, message))) = (
                parse_meta,
                Commit::parse_from_bytes
            ).parse(&bytes)
            .map_err(|e|GitError::invalid_commit(&e.to_string()))?;
        Ok(Commit {
            tree_hash:   String::from_utf8(tree_hash.to_vec())?,
            parent_hash: parent_hash.map(|parent|String::from_utf8_lossy(parent).into_owned()),
            author:      String::from_utf8(author.to_vec())?,
            committer:   String::from_utf8(committer.to_vec())?,
            message:     String::from_utf8(message.to_vec())?,
        })
    }
}

impl From<Commit> for Vec<u8> {
    fn from(commit: Commit) -> Vec<u8> {
        let parent_line = if let Some(hash) = commit.parent_hash.map(|hash| format!("parent {}\n", hash)) {
                hash
            } else {
                "".to_owned()
            };
        format!("tree {}\n{}\
                author {}\n\
                committer {}\n\
                \n\
                {}",
            commit.tree_hash,
            parent_line,
            commit.author,
            commit.committer,
            commit.message,
        ).into_bytes()
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parent_line = if let Some(hash) = self.parent_hash.clone().map(|hash| format!("parent {}\n", hash)) {
                hash
            } else {
                "".to_owned()
            };
        write!(f, "tree {}\n{}\
                   author {}\n\
                   committer {}\n\
                   \n\
                   {}",
                self.tree_hash,
                parent_line,
                self.author,
                self.committer,
                self.message,
        )
    }
}
