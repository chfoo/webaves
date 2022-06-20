mod manpage;

use clap::{crate_name, Command};

fn main() -> anyhow::Result<()> {
    let command = Command::new(crate_name!())
        .subcommand_required(true)
        .subcommand(Command::new("gen-man-page").about("Generate man pages into docs/"));

    let arg_matches = command.get_matches();

    match arg_matches.subcommand() {
        Some(("gen-man-page", _sub_matches)) => crate::manpage::handle_manpage_command()?,
        _ => unreachable!(),
    };

    Ok(())
}
