use std::io::Write;

use regex::Regex;

pub fn handle_docs_command() -> anyhow::Result<()> {
    let docs_dir = crate::common::root_project_dir().join("docs");

    eprintln!("Docs path: {docs_dir:?}");

    let mut process = std::process::Command::new("make")
        .arg("html")
        .current_dir(&docs_dir)
        .spawn()?;
    let exit_status = process.wait()?;
    anyhow::ensure!(exit_status.success());

    Ok(())
}

struct LicenseInfo {
    name: String,
    license: String,
    authors: String,
    url: String,
}

pub fn handle_gen_copyright_file_command() -> anyhow::Result<()> {
    let metadata = crate::common::cargo_metadata();
    let output_path = metadata.target_directory.join("xtask/copyright.txt");

    eprintln!("Output path: {output_path:?}");

    let mut license_infos = Vec::new();

    for package in &metadata.packages {
        if package.name == "xtask" {
            continue;
        }

        license_infos.push(LicenseInfo {
            name: package.name.to_string(),
            license: package.license.to_owned().unwrap_or_default(),
            authors: package
                .authors
                .iter()
                .map(|s| remove_email(s))
                .collect::<Vec<String>>()
                .join(", ")
                .to_string(),
            url: package
                .homepage
                .to_owned()
                .unwrap_or_else(|| package.repository.to_owned().unwrap_or_default()),
        });
    }

    license_infos.sort_by(|a, b| a.name.cmp(&b.name));
    license_infos.dedup_by(|a, b| a.name == b.name);

    let mut output_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(output_path)?;

    output_file.write_all(
        b"This application contains software used under license including but not limited to:\n\n",
    )?;

    for license_info in &license_infos {
        writeln!(output_file, "{}", license_info.name)?;
        writeln!(output_file, "    {}", license_info.authors)?;
        writeln!(output_file, "    {}", license_info.license)?;
        writeln!(output_file, "    {}", license_info.url)?;
        writeln!(output_file)?;
    }

    let extra_path = crate::common::root_project_dir().join("xtask/data/copyright_libs.txt");
    output_file.write_all(&std::fs::read(extra_path)?)?;

    Ok(())
}

fn remove_email(text: &str) -> String {
    let re = Regex::new(r" <.+@.+>").unwrap();

    re.replace_all(text, r"").into_owned()
}
