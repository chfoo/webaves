use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Datelike, Timelike, Utc};
use flate2::{write::GzEncoder, Compression};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

pub fn handle_package_app_command(target_triple: Option<&str>) -> anyhow::Result<()> {
    let host_target_triple = crate::common::host_target_triple();
    let binary_target_triple = target_triple.unwrap_or(&host_target_triple);
    let target_dir = crate::common::target_dir();
    let package_dir = target_dir.join("xtask/package/").join(binary_target_triple);

    eprintln!("Output path: {package_dir:?}");

    if package_dir.exists() {
        std::fs::remove_dir_all(&package_dir)?;
    }
    std::fs::create_dir_all(&package_dir)?;

    copy_project_file("README.md", "README.md", &package_dir)?;
    copy_project_file("LICENSE", "MPL-2.0.txt", &package_dir)?;
    copy_file(
        &target_dir.join("xtask/copyright.txt"),
        &package_dir.join("copyright.txt"),
    )?;

    let binary_path = crate::common::binary_path("webaves-app", target_triple, true);
    let mut dest_binary_path = package_dir.join(binary_path.file_name().unwrap());
    set_path_basename(&mut dest_binary_path, "webaves");
    copy_file(&binary_path, &dest_binary_path)?;

    if binary_target_triple.contains("windows") || binary_target_triple.contains("apple") {
        create_zip_package(&package_dir, binary_target_triple)?;
    } else {
        create_targz_package(&package_dir, binary_target_triple)?;
    }

    Ok(())
}

fn copy_file(source: &Path, dest: &Path) -> std::io::Result<()> {
    eprintln!("Copy {source:?} -> {dest:?}");
    std::fs::copy(source, dest)?;
    Ok(())
}

fn copy_project_file<S: Into<PathBuf>, N: Into<PathBuf>>(
    source: S,
    name: N,
    dest_dir: &Path,
) -> std::io::Result<()> {
    let source = crate::common::root_project_dir().join(source.into());
    let dest = dest_dir.join(name.into());
    copy_file(&source, &dest)
}

fn create_zip_package(source_dir: &Path, target_triple: &str) -> anyhow::Result<()> {
    let target_dir = crate::common::target_dir();
    let zip_dir = target_dir.join("xtask/zip");
    std::fs::create_dir_all(&zip_dir)?;

    let zip_path = zip_dir.join(format!(
        "webaves-{}-{}-{}.zip",
        crate::common::version("webaves-app")?,
        target_triple,
        Utc::now().timestamp()
    ));
    zip_directory(source_dir, &zip_path)?;

    Ok(())
}

fn zip_directory(dir_path: &Path, dest_file: &Path) -> anyhow::Result<()> {
    eprintln!("Create zip {dir_path:?} -> {dest_file:?}");

    let zip_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_file)?;
    let mut zip = ZipWriter::new(zip_file);
    let walker = WalkDir::new(dir_path);

    for entry in walker {
        let entry = entry?;

        if entry.path().is_file() {
            let stripped = entry.path().strip_prefix(dir_path)?;
            let entry_name = stripped.to_string_lossy();

            eprintln!("Deflate {:?} -> {:?}", entry.path(), entry_name);

            let mut options = FileOptions::default().compression_level(Some(9));

            let entry_metadata = entry.metadata()?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                options = options.unix_permissions(entry_metadata.permissions().mode());
            }

            let modified = DateTime::<Utc>::from(entry_metadata.modified()?);
            options = options.last_modified_time(
                zip::DateTime::from_date_and_time(
                    modified.year() as u16,
                    modified.month() as u8,
                    modified.day() as u8,
                    modified.hour() as u8,
                    modified.minute() as u8,
                    modified.second() as u8,
                )
                .unwrap(),
            );

            zip.start_file(entry_name, options)?;

            let mut source_file = OpenOptions::new().read(true).open(entry.path())?;
            std::io::copy(&mut source_file, &mut zip)?;
        }
    }

    zip.finish()?;

    Ok(())
}

fn create_targz_package(source_dir: &Path, target_triple: &str) -> anyhow::Result<()> {
    let target_dir = crate::common::target_dir();
    let zip_dir = target_dir.join("xtask/zip");
    std::fs::create_dir_all(&zip_dir)?;

    let zip_path = zip_dir.join(format!(
        "webaves-{}-{}-{}.tar.gz",
        crate::common::version("webaves-app")?,
        target_triple,
        Utc::now().timestamp()
    ));
    targz_directory(source_dir, &zip_path)?;

    Ok(())
}

fn targz_directory(dir_path: &Path, dest_file: &Path) -> anyhow::Result<()> {
    eprintln!("Create tar.gz {dir_path:?} -> {dest_file:?}");

    let gz_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_file)?;
    let gz_file = GzEncoder::new(gz_file, Compression::new(9));
    let mut tar = tar::Builder::new(gz_file);
    let walker = WalkDir::new(dir_path);

    for entry in walker {
        let entry = entry?;

        if entry.path().is_file() {
            let entry_name = entry.path().strip_prefix(dir_path)?;

            eprintln!("tar.gzip {:?} -> {:?}", entry.path(), entry_name);

            let mut source_file = OpenOptions::new().read(true).open(entry.path())?;
            tar.append_file(entry_name, &mut source_file)?;
        }
    }

    tar.into_inner()?.finish()?;

    Ok(())
}

fn set_path_basename(path: &mut PathBuf, new_basename: &str) {
    let extension = path.extension().map(|p| p.to_os_string());
    path.pop();
    path.push(new_basename);

    if let Some(extension) = extension {
        path.set_extension(extension);
    }
}
