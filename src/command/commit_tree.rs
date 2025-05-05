use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use clap::{Parser, Subcommand};
use crate::utils::{
    zlib::compress_object,
    hash::hash_object,
};
use crate::{
    GitError,
    Result,
};
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "commit-tree", about = "Create a commit object from a tree object")]
pub struct CommitTree {
    #[arg(required = true, help = "The tree object hash")]
    tree_hash: String,

    #[arg(short = 'm', required = true, help = "The commit message")]
    message: String,

    #[arg(short = 'p', help = "The parent commit hash")]
    pcommit: Option<String>,
}

impl CommitTree {
    fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(CommitTree::try_parse_from(args)?))
    }

    fn get_author_info() -> (String, String) {
        let author_name = env::var("GIT_AUTHOR_NAME").unwrap_or_else(|_| "Default Name".to_string());
        let author_email = env::var("GIT_AUTHOR_EMAIL").unwrap_or_else(|_| "default_email@example.com".to_string());
        (author_name, author_email)
    }

    fn build_commit_content(&self) -> Result<String> {
        let (author_name, author_email) = Self::get_author_info();

        let mut content = format!("tree {}\n", self.tree_hash);

        if let Some(parent) = &self.pcommit {
            content.push_str(&format!("parent {}\n", parent));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timezone = "+0000"; 

        content.push_str(&format!(
            "author {} <{}> {} {}\n",
            author_name, author_email, timestamp, timezone
        ));
        content.push_str(&format!(
            "committer {} <{}> {} {}\n\n",
            author_name, author_email, timestamp, timezone
        ));

        content.push_str(&self.message);

        Ok(content)
    }

    fn write_commit_object(&self, commit_content: String) -> Result<String> {
        let commit_hash = hash_object(commit_content.as_bytes().to_vec(), "commit")?;

        let mut objpath = PathBuf::from(".git/objects");
        objpath.push(&commit_hash[0..2]);
        objpath.push(&commit_hash[2..]);

        std::fs::create_dir_all(objpath.parent().unwrap()).map_err(|_| GitError::FileNotFound("commit path not found".to_string()))?;
        let compressed = compress_object(commit_content.into_bytes())?; 
        std::fs::write(&objpath, compressed)?;

        Ok(commit_hash)
    }
}

impl SubCommand for CommitTree {
    fn run(&self) -> Result<i32> {
        let commit_content = self.build_commit_content()?;

        let commit_hash = self.write_commit_object(commit_content)?;

        println!("{}", commit_hash);

        Ok(0)
    }
}