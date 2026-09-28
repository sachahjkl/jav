use anyhow::{anyhow, bail, Context, Result};

use crate::cli::UpgradeArgs;
use crate::output;
use crate::upgrade;

pub fn run(args: UpgradeArgs) -> Result<()> {
    let executable = upgrade::current_executable()?;
    upgrade::ensure_supported_host(&executable)?;

    let client = upgrade::build_client()?;
    let options = upgrade::UpdateOptions::from_env();
    let release = upgrade::fetch_latest_release(&client, &options)?;
    let manifest = upgrade::download_manifest(&client, &release, &options.asset_name)?;

    if args.check {
        output::status("current", upgrade::current_version());
        for line in upgrade::release_summary(&manifest) {
            output::status("release", line);
        }
        return Ok(());
    }

    if !upgrade::is_newer_version(upgrade::current_version(), &manifest.version)? {
        output::status("current", "no newer release available");
        return Ok(());
    }

    let rid = match args.rid {
        Some(rid) => rid,
        None => upgrade::detect_rid()?.to_string(),
    };

    let asset = manifest
        .assets
        .iter()
        .find(|asset| asset.rid.eq_ignore_ascii_case(&rid))
        .ok_or_else(|| anyhow!("no release asset found for rid {rid}"))?;

    if asset.url.trim().is_empty() {
        bail!("release.json is missing assets[].url for downloadable upgrade assets");
    }

    let bytes = client
        .get(&asset.url)
        .send()
        .context("failed to download release asset")?
        .error_for_status()
        .context("release asset request failed")?
        .bytes()
        .context("failed to read release asset")?;

    upgrade::verify_checksum(&bytes, &asset.sha256)?;
    let replacement = upgrade::prepare_replacement(&executable, &asset.file_name, &bytes)?;
    upgrade::replace_executable(&executable, replacement)?;

    output::status(
        "upgraded",
        format!("{}+{}", manifest.version, manifest.commit),
    );
    Ok(())
}
