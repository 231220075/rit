mod cli;
mod utils;
mod command;

use cli::args;
use std::env;
#[allow(unused)]
use crate::utils::error::{
    Result,
    GitError,
};

fn main() -> Result<()> {
    /*  later to change to Args::get_from_cli()
     *  let args = Args::get_from_cli();
    */

    let args = args::get_args(env::args().skip(1));
    match args {
        Ok(cmd) => cli::command::git_execute(cmd),
        Err(err) => {
            eprintln!("{err}");
            Ok(())
        }
    }
}
