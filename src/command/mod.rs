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

pub use init::Init;
pub use add::Add;
pub use rm::Rm;
pub use commit::Commit;

#[allow(unused)]
use crate::{Result, GitError};

pub trait SubCommand {
    fn run(&self) -> Result<()>;
}
