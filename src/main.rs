mod cli{
    pub mod args;
    pub mod command;
}
fn main() {
    cli::command::git_execute();
}
