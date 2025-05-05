/// front-end command
/// offen used by users
pub mod add;
pub mod branch;
pub mod checkout;
pub mod commit;
pub mod fetch;
pub mod init;
pub mod merge;
pub mod pull;
pub mod push;
pub mod rm;

/// plumbing command
/// used internaly by git
/// #reference
/// - [plumbind commands](https: //git-scm.com/book/en/v2/Appendix-C:-Git-Commands-Plumbing-Commands)
pub mod cat_file;
pub mod hash_object;
pub mod update_index;
pub mod write_tree;
pub mod commit_tree;

pub use init::Init;
pub use add::Add;
pub use rm::Rm;
pub use commit::Commit;
pub use cat_file::CatFile;
pub use hash_object::HashObject;


#[allow(unused)]
use crate::{Result, GitError};
use std::fmt::Debug;

pub trait SubCommand: Debug {
    fn run(&self) -> Result<i32>;
}

