mod common;
mod docs;
mod git;
mod github;
mod manpage;
mod package;

use std::path::PathBuf;

use clap::{crate_name, Arg, Command};

const GEN_MAN_PAGE_ABOUT: &str = "Generate man pages files and fragments into docs/ directory.

The fragment files are to be included in written documentation. \
They're manually generated and committed so that they can be rendered \
on readthedocs.org.

Required software: pandoc";
const BUILD_HTML_DOCS_ABOUT: &str = "Build HTML files in the docs/ directory.

Required software: sphinx-doc
Required Python packages: myst-parser";
const GEN_COPYRIGHT_FILE_ABOUT: &str =
    "Generate a copyright & license file listing dependencies in target/xtask/copyright.txt";
const GIT_TAG_ABOUT: &str = "Runs git tag command with the appropriate name for a crate.";
const BUILD_PACKAGE_APP_ABOUT: &str = "Packages the application into a simple zip/tar.gz archive.

The binary, readme file, license file are included. \
This command expects the requisite files to be already built.";
const GET_GH_ARTIFACTS_ABOUT: &str = "Download the release binaries generated by GitHub Actions.

The artifacts will be placed in target/xtask/github_artifacts/.";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::new(crate_name!())
        .subcommand_required(true)
        .subcommand(Command::new("gen-man-page").long_about(GEN_MAN_PAGE_ABOUT))
        .subcommand(Command::new("build-html-docs").long_about(BUILD_HTML_DOCS_ABOUT))
        .subcommand(Command::new("gen-copyright-file").long_about(GEN_COPYRIGHT_FILE_ABOUT))
        .subcommand(
            Command::new("git-tag").long_about(GIT_TAG_ABOUT).arg(
                Arg::new("crate")
                    .takes_value(true)
                    .required(true)
                    .help("Name of crate"),
            ),
        )
        .subcommand(
            Command::new("package-app")
                .long_about(BUILD_PACKAGE_APP_ABOUT)
                .arg(
                    Arg::new("target_triple")
                        .long("target-triple")
                        .takes_value(true),
                ),
        )
        .subcommand(
            Command::new("get-artifacts")
                .long_about(GET_GH_ARTIFACTS_ABOUT)
                .arg(
                    Arg::new("key_file")
                        .long("key-file")
                        .short('k')
                        .takes_value(true)
                        .required(true)
                        .value_parser(clap::value_parser!(PathBuf))
                        .help("Path to file containing personal access token"),
                )
                .arg(
                    Arg::new("run_id")
                        .long("run-id")
                        .short('r')
                        .takes_value(true)
                        .required(true)
                        .help("Actions run ID"),
                ),
        );

    let arg_matches = command.get_matches();

    match arg_matches.subcommand() {
        Some(("gen-man-page", _sub_matches)) => crate::manpage::handle_manpage_command()?,
        Some(("build-html-docs", _sub_matches)) => crate::docs::handle_docs_command()?,
        Some(("gen-copyright-file", _sub_matches)) => {
            crate::docs::handle_gen_copyright_file_command()?
        }
        Some(("git-tag", sub_matches)) => {
            crate::git::handle_git_tag_command(sub_matches.get_one::<String>("crate").unwrap())?
        }
        Some(("package-app", sub_matches)) => crate::package::handle_package_app_command(
            sub_matches
                .get_one::<String>("target_triple")
                .map(|s| s.as_str()),
        )?,
        Some(("get-artifacts", sub_matches)) => {
            crate::github::handle_get_artifacts_command(
                sub_matches.get_one::<PathBuf>("key_file").unwrap(),
                sub_matches.get_one::<String>("run_id").unwrap(),
            )
            .await?
        }
        _ => unreachable!(),
    };

    Ok(())
}
