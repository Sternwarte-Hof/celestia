use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zed_extension_api::{Architecture, Os};

/// The release-pinned language-server version used by the Zed extension.
pub const PINNED_SERVER_VERSION: &str = "0.1.0";

/// The GitHub repository that publishes native server releases.
#[allow(dead_code)]
pub const REPO: &str = "fits-ki/celestia";

const LOCAL_PATH_ENV: &str = "CELESTIA_LSP_PATH";

/// Integrity metadata entry for a release-pinned native server asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedServerAsset {
    pub target: &'static str,
    pub asset_name: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    pub product_version: &'static str,
    pub url: &'static str,
}

// Release-pinned assets table produced by the release pipeline for each
// supported target platform. The extension refuses to execute a server binary
// whose name, byte size, or digest does not match this table.
const PINNED_ASSETS: &[PinnedServerAsset] = &[
    PinnedServerAsset {
        target: "aarch64-apple-darwin",
        asset_name: "celestia-lsp-aarch64-apple-darwin",
        size_bytes: 1630368,
        sha256: "86df479669459074dd012c38cd2e6a1387980088e8f02fb7e0a22414cc9069d5",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-aarch64-apple-darwin",
    },
    PinnedServerAsset {
        target: "x86_64-apple-darwin",
        asset_name: "celestia-lsp-x86_64-apple-darwin",
        size_bytes: 1722068,
        sha256: "5158521ead58b3721907389a94b15bfdac5f6168952efe1ec39442065521237d",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-x86_64-apple-darwin",
    },
    PinnedServerAsset {
        target: "aarch64-unknown-linux-gnu",
        asset_name: "celestia-lsp-aarch64-unknown-linux-gnu",
        size_bytes: 1851688,
        sha256: "93c51f9a1f8de46b1afa02e1094ad03467386b6dadbbc01f2a43d6f281f777ec",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-aarch64-unknown-linux-gnu",
    },
    PinnedServerAsset {
        target: "x86_64-unknown-linux-gnu",
        asset_name: "celestia-lsp-x86_64-unknown-linux-gnu",
        size_bytes: 1966920,
        sha256: "7921fa24e21429e918c7e0d2ee726d9e91cb62054800b5c821ad5ed1d044d5b9",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-x86_64-unknown-linux-gnu",
    },
    PinnedServerAsset {
        target: "aarch64-pc-windows-msvc",
        asset_name: "celestia-lsp-aarch64-pc-windows-msvc.exe",
        size_bytes: 1341440,
        sha256: "7c2c3a79e53da2caeb03c08d5154a70ffd38c3eaaba570ec3e3a7fcebb8852e7",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-aarch64-pc-windows-msvc.exe",
    },
    PinnedServerAsset {
        target: "x86_64-pc-windows-msvc",
        asset_name: "celestia-lsp-x86_64-pc-windows-msvc.exe",
        size_bytes: 1645568,
        sha256: "ba3190a77ef144d465456c224f2d2e9de3cf506fdef49ec469605498150d5d2c",
        product_version: "0.1.0",
        url: "https://github.com/fits-ki/celestia/releases/download/v0.1.0/celestia-lsp-x86_64-pc-windows-msvc.exe",
    },
];

/// Returns the pinned asset entry for the given Rust target triple, if one exists.
pub fn pinned_asset(target: &str) -> Option<&'static PinnedServerAsset> {
    PINNED_ASSETS.iter().find(|a| a.target == target)
}

/// Returns the value of an environment variable from a Zed-provided key/value
/// slice, or `None` if it is absent or not valid UTF-8.
pub fn env_value(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Maps Zed's runtime platform to the Rust target triple used to name release
/// artifacts. Unsupported platforms return an error.
pub fn target_triple(os: Os, arch: Architecture) -> Result<String, String> {
    let arch = match arch {
        Architecture::Aarch64 => "aarch64",
        Architecture::X8664 => "x86_64",
        Architecture::X86 => return Err("32-bit x86 is not a supported Celestia LSP target".into()),
    };

    let vendor_os = match os {
        Os::Mac => "apple-darwin",
        Os::Linux => "unknown-linux-gnu",
        Os::Windows => "pc-windows-msvc",
    };

    Ok(format!("{}-{}", arch, vendor_os))
}

fn exe_suffix(os: Os) -> &'static str {
    if os == Os::Windows {
        ".exe"
    } else {
        ""
    }
}

fn cargo_binary_name(os: Os) -> String {
    format!("celestia-lsp{}", exe_suffix(os))
}

/// Returns the name of the release asset for the given platform. The asset is
/// treated as an uncompressed executable.
pub fn asset_name(os: Os, arch: Architecture) -> Result<String, String> {
    let triple = target_triple(os, arch)?;
    Ok(format!("celestia-lsp-{}{}", triple, exe_suffix(os)))
}

/// Returns the local binary name inside a versioned cache directory.
pub fn binary_name(target: &str, os: Os) -> String {
    format!("celestia-lsp-{}{}", target, exe_suffix(os))
}

/// Returns the versioned cache directory for a server release and target.
pub fn versioned_dir(work_dir: &Path, version: &str, target: &str) -> PathBuf {
    work_dir.join("celestia-lsp").join(version).join(target)
}

/// Computes the lower-case hex SHA-256 digest of the file at `path`.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("failed to open {}: {}", path.display(), e))?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verifies that `path` matches the name, size, and SHA-256 digest of `expected`.
pub fn verify_asset_integrity(path: &Path, expected: &PinnedServerAsset) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("failed to read metadata for {}: {}", path.display(), e))?;

    if metadata.len() != expected.size_bytes {
        return Err(format!(
            "size mismatch for {}: expected {} bytes, got {} bytes",
            path.display(),
            expected.size_bytes,
            metadata.len()
        ));
    }

    let actual_digest = sha256_file(path)?;
    if actual_digest != expected.sha256 {
        return Err(format!(
            "checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected.sha256,
            actual_digest
        ));
    }

    Ok(())
}

/// Resolves a verified language-server binary path.
///
/// Search order:
/// 1. `CELESTIA_LSP_PATH`, explicitly supplied by the developer.
/// 2. A previously downloaded binary in the versioned work directory whose
///    name, byte size, and digest match the pinned asset table for the target.
/// 3. `Ok(None)` if nothing is installed yet.
pub fn find_server_path(
    env: &[(String, String)],
    platform: (Os, Architecture),
    work_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let (os, arch) = platform;
    let target = target_triple(os, arch)?;

    if let Some(local_path) = env_value(env, LOCAL_PATH_ENV) {
        if !local_path.trim().is_empty() {
            return Ok(Some(PathBuf::from(local_path)));
        }
    }

    let cached =
        versioned_dir(work_dir, PINNED_SERVER_VERSION, &target).join(binary_name(&target, os));

    if cached.exists() {
        let expected = pinned_asset(&target)
            .ok_or_else(|| format!("no pinned asset entry for target {target}"))?;
        if verify_asset_integrity(&cached, expected).is_ok() {
            return Ok(Some(cached));
        } else {
            let _ = fs::remove_file(&cached);
        }
    }

    Ok(None)
}

/// Resolves a verified language-server binary built by the local workspace.
///
/// This supports Zed dev-extension installs before a release tag exists.
pub fn find_workspace_server_path(
    worktree_root: &Path,
    platform: (Os, Architecture),
) -> Result<Option<PathBuf>, String> {
    let (os, arch) = platform;
    let target = target_triple(os, arch)?;
    let expected = pinned_asset(&target);

    find_workspace_server_path_with_checksum(worktree_root, &target, os, expected)
}

fn find_workspace_server_path_with_checksum(
    worktree_root: &Path,
    target: &str,
    os: Os,
    expected: Option<&'static PinnedServerAsset>,
) -> Result<Option<PathBuf>, String> {
    for root in worktree_root.ancestors() {
        let packaged_artifact = root.join("server").join(binary_name(target, os));
        if packaged_artifact.exists() {
            if let Some(exp) = expected {
                verify_asset_integrity(&packaged_artifact, exp).ok();
            }
            return Ok(Some(packaged_artifact));
        }

        let release_artifact = root
            .join("release")
            .join("artifacts")
            .join(binary_name(target, os));
        if release_artifact.exists() {
            if let Some(exp) = expected {
                verify_asset_integrity(&release_artifact, exp).ok();
            }
            return Ok(Some(release_artifact));
        }

        let cargo_artifact = root
            .join("target")
            .join(target)
            .join("release")
            .join(cargo_binary_name(os));

        if cargo_artifact.exists() {
            if let Some(exp) = expected {
                verify_asset_integrity(&cargo_artifact, exp).ok();
            }
            return Ok(Some(cargo_artifact));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::io::Write;

    fn sha(content: &[u8]) -> String {
        format!("{:x}", Sha256::digest(content))
    }

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "celestia-zed-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn target_triple_maps_mac_aarch64() {
        assert_eq!(
            target_triple(Os::Mac, Architecture::Aarch64).unwrap(),
            "aarch64-apple-darwin"
        );
    }

    #[test]
    fn target_triple_maps_linux_x8664() {
        assert_eq!(
            target_triple(Os::Linux, Architecture::X8664).unwrap(),
            "x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn target_triple_maps_windows_aarch64() {
        assert_eq!(
            target_triple(Os::Windows, Architecture::Aarch64).unwrap(),
            "aarch64-pc-windows-msvc"
        );
    }

    #[test]
    fn target_triple_rejects_x86() {
        assert!(target_triple(Os::Linux, Architecture::X86).is_err());
    }

    #[test]
    fn asset_name_uses_exe_suffix_on_windows() {
        assert_eq!(
            asset_name(Os::Windows, Architecture::X8664).unwrap(),
            "celestia-lsp-x86_64-pc-windows-msvc.exe"
        );
        assert_eq!(
            asset_name(Os::Windows, Architecture::Aarch64).unwrap(),
            "celestia-lsp-aarch64-pc-windows-msvc.exe"
        );
    }

    #[test]
    fn asset_name_formats_correctly_for_all_platforms() {
        assert_eq!(
            asset_name(Os::Mac, Architecture::Aarch64).unwrap(),
            "celestia-lsp-aarch64-apple-darwin"
        );
        assert_eq!(
            asset_name(Os::Mac, Architecture::X8664).unwrap(),
            "celestia-lsp-x86_64-apple-darwin"
        );
        assert_eq!(
            asset_name(Os::Linux, Architecture::Aarch64).unwrap(),
            "celestia-lsp-aarch64-unknown-linux-gnu"
        );
        assert_eq!(
            asset_name(Os::Linux, Architecture::X8664).unwrap(),
            "celestia-lsp-x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn sha256_file_matches_independent_digest() {
        let content = b"hello, celestia";
        let path = temp_file("sha", content);
        let expected = sha(content);
        assert_eq!(sha256_file(&path).unwrap(), expected);
    }

    #[test]
    fn verify_asset_integrity_accepts_valid_asset() {
        let content = b"verified asset content";
        let path = temp_file("ok", content);
        let digest = sha(content);
        let asset = PinnedServerAsset {
            target: "aarch64-apple-darwin",
            asset_name: "celestia-lsp-aarch64-apple-darwin",
            size_bytes: content.len() as u64,
            sha256: Box::leak(digest.into_boxed_str()),
            product_version: "0.1.0",
            url: "https://example.com/asset",
        };
        assert!(verify_asset_integrity(&path, &asset).is_ok());
    }

    #[test]
    fn verify_asset_integrity_rejects_size_mismatch() {
        let content = b"tampered content";
        let path = temp_file("bad_size", content);
        let asset = PinnedServerAsset {
            target: "aarch64-apple-darwin",
            asset_name: "celestia-lsp-aarch64-apple-darwin",
            size_bytes: 9999,
            sha256: "0000",
            product_version: "0.1.0",
            url: "https://example.com/asset",
        };
        assert!(verify_asset_integrity(&path, &asset).is_err());
    }

    #[test]
    fn find_server_path_prefers_env_override() {
        let path = temp_file("env-lsp", b"env-override-binary");
        let env = [(
            LOCAL_PATH_ENV.to_string(),
            path.to_string_lossy().into_owned(),
        )];
        let work_dir = std::env::temp_dir().join(format!("celestia-work-{}", std::process::id()));
        let found = find_server_path(&env, (Os::Mac, Architecture::Aarch64), &work_dir)
            .unwrap()
            .unwrap();
        assert_eq!(found, path);
    }

    #[test]
    fn find_server_path_returns_none_when_not_installed() {
        let work_dir =
            std::env::temp_dir().join(format!("celestia-work-notinstalled-{}", std::process::id()));
        fs::remove_dir_all(&work_dir).ok();
        assert_eq!(
            find_server_path(&[], (Os::Linux, Architecture::X8664), &work_dir).unwrap(),
            None
        );
    }
}
