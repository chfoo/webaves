use std::{
    io::Cursor,
    path::PathBuf,
    process::{Command, Stdio},
};

use cargo_metadata::{Message, Metadata, MetadataCommand};
use regex::Regex;

pub fn cargo_command() -> String {
    std::env::var("CARGO").unwrap()
}

pub fn root_project_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::new().join(&manifest_dir).join("..");
    dir.canonicalize().unwrap()
}

pub fn cargo_metadata() -> Metadata {
    MetadataCommand::new().exec().unwrap()
}

pub fn host_target_triple() -> String {
    let process = std::process::Command::new("rustc")
        .arg("-vV")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout).unwrap();

    let re = Regex::new(r"host: (.+)").unwrap();

    let captures = re.captures(&output).expect("host triple not found");
    captures
        .get(1)
        .expect("host triple not found")
        .as_str()
        .to_owned()
}

pub fn target_dir() -> PathBuf {
    let metadata = cargo_metadata();
    metadata.target_directory.as_std_path().to_path_buf()
}

pub fn version(name: &str) -> anyhow::Result<String> {
    for package in &crate::common::cargo_metadata().packages {
        if package.name == name {
            return Ok(package.version.to_string());
        }
    }

    anyhow::bail!("no version for crate {name}");
}

#[allow(dead_code)]
pub fn artifact_path(name: &str) -> anyhow::Result<PathBuf> {
    let process = Command::new(cargo_command())
        .arg("build")
        .arg("--release")
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped())
        .spawn()?;

    let output = process.wait_with_output()?;
    anyhow::ensure!(output.status.success());

    for message in cargo_metadata::Message::parse_stream(Cursor::new(output.stdout)) {
        match message.unwrap() {
            Message::CompilerArtifact(artifact) if artifact.target.name == name => {
                return artifact
                    .executable
                    .map(|p| p.as_std_path().to_owned())
                    .ok_or_else(|| anyhow::anyhow!("diagnostics output has no executable path"))
            }

            _ => (),
        }
    }

    anyhow::bail!("no matching artifact for name {name}")
}

pub fn binary_path(name: &str, target_triple: Option<&str>, release: bool) -> PathBuf {
    let host_target_triple = host_target_triple();
    let mut path = target_dir();

    if let Some(target_triple) = target_triple {
        path.push(target_triple);
    }

    let binary_name = if target_triple
        .unwrap_or(&host_target_triple)
        .contains("windows")
    {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    if release {
        path.push("release");
        path.push(binary_name);
    } else {
        path.push("debug");
        path.push(binary_name);
    }

    path
}
