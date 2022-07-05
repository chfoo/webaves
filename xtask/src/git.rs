use std::process::Command;

pub fn handle_git_tag_command(crate_name: &str) -> anyhow::Result<()> {
    let version = crate::common::version(crate_name)?;
    let tag_name = format!("{}-v{}", crate_name, version);

    let mut process = Command::new("git")
        .arg("tag")
        .arg("--sign")
        .arg(tag_name)
        .spawn()?;

    let status = process.wait()?;

    anyhow::ensure!(status.success());

    Ok(())
}
