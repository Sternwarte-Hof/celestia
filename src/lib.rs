//! Zed Extension entry point for Celestia Authoring Formats.

use std::fs;
use std::path::Path;

use zed_extension_api as zed;

mod server;

struct CelestiaExtension;

fn development_server_command(
    manifest_path: Option<&str>,
    cargo_path: Option<String>,
) -> zed::Result<Option<zed::Command>> {
    let Some(manifest_path) = manifest_path else {
        return Ok(None);
    };
    let cargo_path = cargo_path.ok_or_else(|| {
        "local Celestia extension development requires `cargo` on Zed's PATH".to_string()
    })?;

    Ok(Some(zed::Command {
        command: cargo_path,
        args: vec![
            "run".to_string(),
            "--quiet".to_string(),
            "--manifest-path".to_string(),
            manifest_path.to_string(),
            "--package".to_string(),
            "celestia-lsp".to_string(),
            "--".to_string(),
        ],
        env: vec![],
    }))
}

fn workspace_development_server_command(
    worktree_root: &str,
    server_manifest: Result<String, String>,
    cargo_path: Option<String>,
) -> zed::Result<Option<zed::Command>> {
    let Ok(server_manifest) = server_manifest else {
        return Ok(None);
    };
    if !server_manifest
        .lines()
        .any(|line| line.trim() == "name = \"celestia-lsp\"")
    {
        return Ok(None);
    }

    let manifest_path = Path::new(worktree_root).join("Cargo.toml");
    development_server_command(manifest_path.to_str(), cargo_path)
}

fn report_installation_failure(
    language_server_id: &zed::LanguageServerId,
    message: &str,
) -> String {
    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Failed(message.to_string()),
    );
    message.to_string()
}

impl CelestiaExtension {
    pub fn new() -> Self {
        Self
    }
}

impl zed::Extension for CelestiaExtension {
    fn new() -> Self {
        Self::new()
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command, String> {
        if let Some(command) = development_server_command(
            option_env!("CELESTIA_ZED_LOCAL_DEV_MANIFEST"),
            worktree.which("cargo"),
        )? {
            return Ok(command);
        }

        let (os, arch) = zed::current_platform();
        let env = worktree.shell_env();
        let worktree_root = worktree.root_path();
        let work_dir = std::env::current_dir().map_err(|e| e.to_string())?;

        if let Some(path) = server::find_server_path(&env, (os, arch), &work_dir)? {
            return Ok(zed::Command {
                command: path.to_string_lossy().into_owned(),
                args: vec![],
                env: vec![],
            });
        }

        if let Some(command) = workspace_development_server_command(
            &worktree_root,
            worktree.read_text_file("crates/celestia-lsp/Cargo.toml"),
            worktree.which("cargo"),
        )? {
            return Ok(command);
        }

        if let Some(path) =
            server::find_workspace_server_path(Path::new(&worktree_root), (os, arch))?
        {
            return Ok(zed::Command {
                command: path.to_string_lossy().into_owned(),
                args: vec![],
                env: vec![],
            });
        }

        let target = server::target_triple(os, arch)?;
        let pinned = server::pinned_asset(&target)
            .ok_or_else(|| format!("no pinned asset entry for target {target}"))?;

        let asset_name = server::asset_name(os, arch)?;
        if asset_name != pinned.asset_name {
            return Err(report_installation_failure(
                language_server_id,
                &format!(
                    "asset name mismatch: expected {}, got {}",
                    pinned.asset_name, asset_name
                ),
            ));
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let cached = server::versioned_dir(&work_dir, server::PINNED_SERVER_VERSION, &target)
            .join(server::binary_name(&target, os));

        let relative = cached
            .strip_prefix(&work_dir)
            .unwrap_or(&cached)
            .to_string_lossy()
            .into_owned();

        if let Err(e) = fs::create_dir_all(cached.parent().unwrap_or(Path::new(""))) {
            return Err(report_installation_failure(
                language_server_id,
                &e.to_string(),
            ));
        }

        if let Err(e) =
            zed::download_file(pinned.url, &relative, zed::DownloadedFileType::Uncompressed)
        {
            let _ = fs::remove_file(&cached);
            return Err(report_installation_failure(language_server_id, &e));
        }

        if let Err(e) = zed::make_file_executable(&relative) {
            let _ = fs::remove_file(&cached);
            return Err(report_installation_failure(language_server_id, &e));
        }

        if let Err(e) = server::verify_asset_integrity(&cached, pinned) {
            let _ = fs::remove_file(&cached);
            let msg = format!("integrity failure for {asset_name}: {e}");
            return Err(report_installation_failure(language_server_id, &msg));
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        Ok(zed::Command {
            command: cached.to_string_lossy().into_owned(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(CelestiaExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_creation() {
        let _ext = CelestiaExtension::new();
    }

    #[test]
    fn development_command_runs_workspace_lsp_through_cargo() {
        let manifest_path = "/repo/Cargo.toml";
        let command = development_server_command(
            Some(manifest_path),
            Some("/toolchain/bin/cargo".to_string()),
        )
        .unwrap()
        .expect("development command");

        assert_eq!(command.command, "/toolchain/bin/cargo");
        assert_eq!(
            command.args,
            vec![
                "run".to_string(),
                "--quiet".to_string(),
                "--manifest-path".to_string(),
                manifest_path.to_string(),
                "--package".to_string(),
                "celestia-lsp".to_string(),
                "--".to_string(),
            ]
        );
    }

    #[test]
    fn release_build_does_not_select_the_cargo_development_command() {
        assert!(development_server_command(None, Some("cargo".to_string()))
            .unwrap()
            .is_none());
    }

    #[test]
    fn source_workspace_selects_the_cargo_development_command() {
        let workspace_root = "/repo";
        let command = workspace_development_server_command(
            workspace_root,
            Ok("[package]\nname = \"celestia-lsp\"\n".to_string()),
            Some("/toolchain/bin/cargo".to_string()),
        )
        .unwrap()
        .expect("development command");

        assert_eq!(command.command, "/toolchain/bin/cargo");
        let manifest_path = Path::new(workspace_root)
            .join("Cargo.toml")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            command.args,
            vec![
                "run".to_string(),
                "--quiet".to_string(),
                "--manifest-path".to_string(),
                manifest_path,
                "--package".to_string(),
                "celestia-lsp".to_string(),
                "--".to_string(),
            ]
        );

        assert!(workspace_development_server_command(
            "/other",
            Err("missing".to_string()),
            Some("cargo".to_string()),
        )
        .unwrap()
        .is_none());
    }
}
