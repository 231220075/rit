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

fn main() {
    /*  later to change to Args::get_from_cli()
     *  let args = Args::get_from_cli();
    */

    let result = args::get_args(env::args().skip(1))
                    .and_then(cli::command::git_execute);
    std::process::exit(match result {
        Ok(retval) => retval,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}
