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

    render_command_recursive(&command, "", &mut roff_table)?;

    let output = serde_json::to_string_pretty(&roff_table)?;
    println!("{}", output);

    Ok(())
}

fn render_command_recursive(
    command: &Command,
    key_prefix: &str,
    table: &mut HashMap<String, HashMap<String, String>>,
) -> anyhow::Result<()> {
    let key = format!("{}{}", key_prefix, command.get_name());

    let sections = render_command_sections(&command)?;
    table.insert(key.clone(), sections);

    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }

        let subcommand_key_prefix = format!("{}/", key);
        render_command_recursive(subcommand, &subcommand_key_prefix, table)?;
    }

    Ok(())
}

fn render_command_sections(command: &Command) -> anyhow::Result<HashMap<String, String>> {
    let man = clap_mangen::Man::new(command.clone());
    let mut buffer = Vec::new();
    let mut output = HashMap::new();

    man.render_title(&mut buffer)?;
    output.insert(
        "title".to_string(),
        String::from_utf8_lossy(&buffer).into_owned(),
    );
    buffer.clear();

    man.render_name_section(&mut buffer)?;
    output.insert(
        "name".to_string(),
        String::from_utf8_lossy(&buffer).into_owned(),
    );
    buffer.clear();

    man.render_synopsis_section(&mut buffer)?;
    output.insert(
        "synopsis".to_string(),
        String::from_utf8_lossy(&buffer).into_owned(),
    );
    buffer.clear();

    man.render_description_section(&mut buffer)?;
    output.insert(
        "description".to_string(),
        String::from_utf8_lossy(&buffer).into_owned(),
    );
    buffer.clear();

    if command.get_arguments().any(|a| !a.is_hide_set()) {
        man.render_options_section(&mut buffer)?;
        output.insert(
            "options".to_string(),
            String::from_utf8_lossy(&buffer).into_owned(),
        );
        buffer.clear();
    }

    if command.has_subcommands() {
        man.render_subcommands_section(&mut buffer)?;
        output.insert(
            "subcommands".to_string(),
            String::from_utf8_lossy(&buffer).into_owned(),
        );
        buffer.clear();
    }

    if command.get_after_help().is_some() || command.get_after_long_help().is_some() {
        man.render_extra_section(&mut buffer)?;
        output.insert(
            "extra".to_string(),
            String::from_utf8_lossy(&buffer).into_owned(),
        );
        buffer.clear();
    }

    if command.get_version().is_some() || command.get_long_version().is_some() {
        man.render_version_section(&mut buffer)?;
        output.insert(
            "version".to_string(),
            String::from_utf8_lossy(&buffer).into_owned(),
        );
        buffer.clear();
    }

    if command.get_author().is_some() {
        man.render_authors_section(&mut buffer)?;
        output.insert(
            "authors".to_string(),
            String::from_utf8_lossy(&buffer).into_owned(),
        );
        buffer.clear();
    }

    Ok(output)
}
