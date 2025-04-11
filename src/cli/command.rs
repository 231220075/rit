use super::args;

pub fn git_execute() {
    let args = args::get_args();
    match args.command.as_str() {
        "add" => {
            println!("Adding files to git: {:?}", args.files);
        }
        "commit" => {
            println!("Committing changes with message: {}", args.message);
        }
        "push" => {
            println!("Pushing changes to remote repository");
        }
        _ => {
            println!("Unknown command: {}", args.command);
        }
    }
}