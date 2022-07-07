use std::{collections::HashMap, io::Write, process::Stdio, thread::JoinHandle};

use anyhow::Context;
use regex::Regex;

pub fn handle_gen_commands_command() -> anyhow::Result<()> {
    let process = std::process::Command::new(crate::common::cargo_command())
        .arg("run")
        .arg("--bin")
        .arg("gen_man_page")
        .arg("--features=gen_man_page")
        .stdout(Stdio::piped())
        .spawn()?;

    let output = process.wait_with_output()?;
    anyhow::ensure!(output.status.success());
    let table = serde_json::from_slice::<HashMap<String, HashMap<String, String>>>(&output.stdout)?;

    let fragment_output_dir = crate::common::root_project_dir()
        .join("docs")
        .join("commands")
        .join("fragments");

    eprintln!("Fragments output dir: {:?}", fragment_output_dir);

    anyhow::ensure!(fragment_output_dir.is_dir());

    for (key, sections) in table.iter() {
        let key = replace_program_name(key);
        let key = key.replace('/', ".");

        let mut content = String::new();

        for section_name in [
            "synopsis",
            "description",
            "options",
            "subcommands",
            "extra",
            "version",
            "authors",
        ] {
            if let Some(section_content) = sections.get(section_name) {
                let section_content = replace_program_name(section_content);
                content.push_str(&section_content);
            }
        }

        let fragment_path = fragment_output_dir.join(format!("{}.rst", key));
        eprintln!("Writing {:?}", fragment_path);

        let mut content = reformat_to_fragment(&key, &content)?;
        content.insert_str(0, ".. Automatically generated; do not edit!\n\n");
        std::fs::write(&fragment_path, content)?;
    }

    Ok(())
}

fn reformat_to_fragment(name: &str, input_roff: &str) -> anyhow::Result<String> {
    let mut process = std::process::Command::new("pandoc")
        .arg("-f")
        .arg("man")
        .arg("-t")
        .arg("rst")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = process.stdin.take().unwrap();
    let input_roff = input_roff.to_string();
    let stdin_handle: JoinHandle<Result<(), std::io::Error>> = std::thread::spawn(move || {
        stdin.write_all(input_roff.as_bytes())?;
        Ok(())
    });
    let output = process.wait_with_output()?;
    anyhow::ensure!(output.status.success());
    stdin_handle.join().unwrap()?;

    let rst_content = String::from_utf8(output.stdout)?;
    let command_name = name.split('.').last().context("missing command name")?;
    let rst_content = lowercase_headers(rst_content);
    let rst_content = reformat_man_page_formatted_commands(command_name, rst_content);

    Ok(rst_content)
}

fn replace_program_name(text: &str) -> String {
    text.replace("PROGRAM_NAME", "webaves")
}

fn lowercase_headers(rst_content: String) -> String {
    rst_content
        .replace("SYNOPSIS", "Synopsis")
        .replace("DESCRIPTION", "Description")
        .replace("OPTIONS", "Options")
        .replace("SUBCOMMANDS", "Subcommands")
        .replace("VERSION", "Version")
}

fn reformat_man_page_formatted_commands(command_name: &str, rst_content: String) -> String {
    // regex to replace `program-subcommand-subsubcommand(1)`
    let re = Regex::new(&format!(r"{}-([a-zA-Z_-]+)\(\d+\)", &command_name)).unwrap();

    re.replace_all(&rst_content, r"$1").into_owned()
}
