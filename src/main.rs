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

    let result = args::Git::from_args(env::args()).and_then(|mut g| g.execute());
    std::process::exit(match result {
        Ok(retval) => retval,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}
