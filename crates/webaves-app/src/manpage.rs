#![allow(dead_code)]

use std::collections::HashMap;

use clap::Command;

mod argutil;
mod common;
mod dns_lookup;
mod echo;
mod logging;
mod warc;

fn main() -> anyhow::Result<()> {
    let command = crate::argutil::build_commands().name("PROGRAM_NAME");
    let mut roff_table = HashMap::new();

    let name = command.get_name().to_string();
    roff_table.insert(name.clone(), render_roff(&command)?);

    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }

        let key = format!("{}.{}", name, subcommand.get_name());

        roff_table.insert(key.clone(), render_roff(subcommand)?);

        for subsubcommand in subcommand.get_subcommands() {
            if subsubcommand.is_hide_set() {
                continue;
            }

            let key = format!("{}.{}", key, subsubcommand.get_name());

            roff_table.insert(key.clone(), render_roff(subsubcommand)?);
        }
    }

    let output = serde_json::to_string_pretty(&roff_table)?;
    println!("{}", output);

    Ok(())
}

fn render_roff(command: &Command) -> anyhow::Result<String> {
    let man = clap_mangen::Man::new(command.clone());
    let mut buffer = Vec::new();

    man.render(&mut buffer)?;

    Ok(String::from_utf8(buffer.clone())?)
}
