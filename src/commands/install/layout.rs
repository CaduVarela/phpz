use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::archive::{extract_php_binary, extract_zip, set_executable};
use crate::download;
use crate::extensions;
use crate::php::source_build;
use crate::php::variant;
use crate::php::version::Build;

const DEFAULT_INI: &str = "\
; php.ini managed by phpz
[PHP]
memory_limit = 512M
error_reporting = E_ALL
display_errors = On
date.timezone = UTC
";

/// Give a freshly compiled tree the standard layout, enabling the shared extensions built.
pub(super) fn lay_out_source_build(tree: &Path, plan: &source_build::Plan) -> Result<(), String> {
    let libexec = tree.join("libexec");
    fs::create_dir_all(&libexec).map_err(|error| error.to_string())?;
    fs::rename(tree.join("bin").join("php"), libexec.join("php"))
        .map_err(|error| format!("moving the php binary: {}", error))?;
    write_wrapper_script(&tree.join("bin").join("php"))?;
    fs::create_dir_all(tree.join("conf.d")).map_err(|error| error.to_string())?;
    variant::write_source(tree)?;

    let mut ini = crate::php::ini::IniDocument::parse(DEFAULT_INI);
    let available = extensions::available_shared(tree);
    for name in plan.shared.iter().filter(|name| **name != "opcache") {
        extensions::enable(&mut ini, name, &available, || Ok(BTreeSet::new()))?;
    }
    ini.save(&tree.join("php.ini"))
        .map_err(|error| format!("writing php.ini: {}", error))
}

pub(super) fn populate(build: &Build, directory: &Path) -> Result<(), String> {
    if cfg!(windows) {
        lay_out_windows_build(build, directory)
    } else {
        lay_out_static_build(build, directory)
    }
}

/// Extensions enabled by default in the Windows php.ini, when the build ships them.
const WINDOWS_DEFAULT_EXTENSIONS: &[&str] = &[
    "bz2",
    "curl",
    "exif",
    "fileinfo",
    "gd",
    "gd2",
    "gmp",
    "intl",
    "mbstring",
    "mysqli",
    "openssl",
    "pdo_mysql",
    "pdo_pgsql",
    "pdo_sqlite",
    "pgsql",
    "sockets",
    "sodium",
    "sqlite3",
    "zip",
];

/// Official Windows zip, extracted with a php.ini made from the development template.
fn lay_out_windows_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.zip");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());
    extract_zip(&archive_path, directory)?;
    let _ = fs::remove_file(&archive_path);

    let template_path = directory.join("php.ini-development");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("reading {}: {}", template_path.display(), error))?;

    // PHP 7.0 and 7.1 name extensions by DLL ("extension=php_curl.dll"); 7.2 and later use short names.
    let directives = windows_directives(!build.at_least(7, 2));
    fs::write(
        directory.join("php.ini"),
        uncomment_directives(&template, &directives),
    )
    .map_err(|error| format!("writing php.ini: {}", error))
}

/// The php.ini lines to activate: extension_dir and the default extensions.
pub fn windows_directives(uses_dll_names: bool) -> Vec<String> {
    let mut directives = vec!["extension_dir = \"ext\"".to_string()];
    directives.extend(WINDOWS_DEFAULT_EXTENSIONS.iter().map(|name| {
        if uses_dll_names {
            format!("extension=php_{}.dll", name)
        } else {
            format!("extension={}", name)
        }
    }));
    directives
}

/// Activate `;directive` lines; `extension_dir` also matches the spaced spelling of PHP 7.0 to 7.2.
pub fn uncomment_directives(content: &str, directives: &[String]) -> String {
    content
        .split_inclusive('\n')
        .map(|line| {
            let text = line.trim_end_matches(['\r', '\n']);
            let is_wanted = text
                .strip_prefix(';')
                .map(|rest| rest.split(';').next().unwrap_or("").trim_end())
                .map(|rest| {
                    directives.iter().any(|wanted| {
                        wanted == rest
                            || (wanted.starts_with("extension_dir") && wanted == rest.trim_start())
                    })
                })
                .unwrap_or(false);
            if is_wanted {
                line[1..].trim_start().to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

/// Static build from static-php-cli, laid out with a wrapper script around the binary.
fn lay_out_static_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.tar.gz");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());

    let libexec_directory = directory.join("libexec");
    fs::create_dir_all(&libexec_directory).map_err(|error| error.to_string())?;
    extract_php_binary(&archive_path, &libexec_directory.join("php"))?;
    let _ = fs::remove_file(&archive_path);

    let bin_directory = directory.join("bin");
    fs::create_dir_all(&bin_directory).map_err(|error| error.to_string())?;
    write_wrapper_script(&bin_directory.join("php"))?;

    if let Some(variant) = build.variant {
        variant::write(directory, variant)?;
    }
    fs::write(directory.join("php.ini"), DEFAULT_INI)
        .map_err(|error| format!("writing php.ini: {}", error))?;
    fs::create_dir_all(directory.join("conf.d")).map_err(|error| error.to_string())?;
    Ok(())
}

/// The static binary has its php.ini path compiled in, so a wrapper points it at this version.
fn write_wrapper_script(path: &Path) -> Result<(), String> {
    let script = "\
#!/bin/sh
DIR=\"$(cd \"$(dirname \"$0\")/..\" && pwd -P)\"
export PHPRC=\"$DIR\"
export PHP_INI_SCAN_DIR=\"$DIR/conf.d\"
exec \"$DIR/libexec/php\" \"$@\"
";
    fs::write(path, script).map_err(|error| format!("writing wrapper: {}", error))?;
    set_executable(path)
}
