use super::{Context, Module, RootModuleConfig};

use crate::configs::nodejs::NodejsConfig;
use crate::formatter::StringFormatter;
use crate::utils;

use regex::Regex;
use semver::Version;
use semver::VersionReq;
use serde_json as json;
use std::path::Path;

/// Creates a module with the current Node.js version
///
/// Will display the Node.js version if any of the following criteria are met:
///     - Current directory contains a `.js`, `.mjs` or `.cjs` file
///     - Current directory contains a `.ts` file
///     - Current directory contains a `package.json` or `.node-version` file
///     - Current directory contains a `node_modules` directory
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    let is_js_project = context
        .try_begin_scan()?
        .set_files(&["package.json", ".node-version"])
        .set_extensions(&["js", "mjs", "cjs", "ts"])
        .set_folders(&["node_modules"])
        .is_match();

    let is_esy_project = context
        .try_begin_scan()?
        .set_folders(&["esy.lock"])
        .is_match();

    if !is_js_project || is_esy_project {
        return None;
    }

    let mut module = context.new_module("nodejs");
    let config = NodejsConfig::try_load(module.config);
    let nodejs_version = utils::exec_cmd("node", &["--version"])?.stdout;
    let engines_version = get_engines_version(&context.current_dir);
    let in_engines_range = check_engines_version(&nodejs_version, engines_version);
    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_meta(|var, _| match var {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => {
                    if in_engines_range {
                        Some(Ok(config.style))
                    } else {
                        Some(Ok(config.not_capable_style))
                    }
                }
                _ => None,
            })
            .map(|variable| match variable {
                "version" => Some(Ok(nodejs_version.trim())),
                _ => None,
            })
            .parse(None)
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `nodejs`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

fn get_engines_version(base_dir: &Path) -> Option<String> {
    let json_str = utils::read_file(base_dir.join("package.json")).ok()?;
    let package_json: json::Value = json::from_str(&json_str).ok()?;
    let raw_version = package_json.get("engines")?.get("node")?.as_str()?;
    Some(raw_version.to_string())
}

fn check_engines_version(nodejs_version: &str, engines_version: Option<String>) -> bool {
    if engines_version.is_none() {
        return true;
    }
    let r = match VersionReq::parse(&engines_version.unwrap()) {
        Ok(r) => r,
        Err(_e) => return true,
    };
    let re = Regex::new(r"\d+\.\d+\.\d+").unwrap();
    let version = re
        .captures(nodejs_version)
        .unwrap()
        .get(0)
        .unwrap()
        .as_str();
    let v = match Version::parse(version) {
        Ok(v) => v,
        Err(_e) => return true,
    };
    r.matches(&v)
}

#[cfg(test)]
mod tests {
    use crate::test::ModuleRenderer;
    use ansi_term::Color;
    use std::fs::{self, File};
    use std::io;
    use std::io::Write;

    #[test]
    fn folder_without_node_files() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = None;
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_package_json() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("package.json"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_package_json_and_esy_lock() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("package.json"))?.sync_all()?;
        let esy_lock = dir.path().join("esy.lock");
        fs::create_dir_all(&esy_lock)?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = None;
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_node_version() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join(".node-version"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_js_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("index.js"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_mjs_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("index.mjs"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_cjs_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("index.cjs"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_ts_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("index.ts"))?.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_node_modules() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let node_modules = dir.path().join("node_modules");
        fs::create_dir_all(&node_modules)?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn engines_node_version_match() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut file = File::create(dir.path().join("package.json"))?;
        file.write_all(
            b"{
            \"engines\":{
                \"node\":\">=12.0.0\"
            }
        }",
        )?;
        file.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Green.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn engines_node_version_not_match() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut file = File::create(dir.path().join("package.json"))?;
        file.write_all(
            b"{
            \"engines\":{
                \"node\":\"<12.0.0\"
            }
        }",
        )?;
        file.sync_all()?;

        let actual = ModuleRenderer::new("nodejs").path(dir.path()).collect();
        let expected = Some(format!("via {} ", Color::Red.bold().paint("⬢ v12.0.0")));
        assert_eq!(expected, actual);
        dir.close()
    }
}
