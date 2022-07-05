use std::{io::Cursor, path::Path};

use anyhow::Context;
use octocrab::{
    models::{ArtifactId, RunId},
    params::actions::ArchiveFormat,
};

const OWNER: &str = "chfoo";
const REPO: &str = "webaves";

pub async fn handle_get_artifacts_command(key_file: &Path, run_id: &str) -> anyhow::Result<()> {
    let dest_dir = crate::common::target_dir().join("xtask/github_artifacts");
    std::fs::create_dir_all(&dest_dir)?;

    let api_key = std::fs::read_to_string(key_file).context("Read API key file")?;
    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(api_key.trim().to_string())
        .build()?;

    eprintln!("List {run_id}");
    let artifacts = octocrab
        .actions()
        .list_workflow_run_artifacts(OWNER, REPO, RunId(run_id.parse()?))
        .send()
        .await?;

    let artifact_ids = artifacts
        .value
        .unwrap()
        .items
        .iter()
        .map(|a| a.id)
        .collect::<Vec<ArtifactId>>();

    for artifact_id in artifact_ids {
        eprintln!("Download {artifact_id}");

        let zip = octocrab
            .actions()
            .download_artifact(OWNER, REPO, artifact_id, ArchiveFormat::Zip)
            .await?;

        if zip.starts_with(b"{") {
            anyhow::bail!("Not a zip: {}", String::from_utf8_lossy(&zip));
        }

        let mut zip = zip::ZipArchive::new(Cursor::new(zip))?;

        zip.extract(&dest_dir)?;
    }

    Ok(())
}
