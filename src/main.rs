mod cli{
    pub mod args;
    pub mod command;
}
mod utils{
    pub mod error;
    pub mod fs;
    pub mod hash;
}
fn main() {
    cli::command::git_execute();
}
