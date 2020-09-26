use std::env::{current_dir, current_exe};
use std::fs;
use std::io::Read;
use std::io::Write as _;
use std::path::Path;

use serde::Deserialize;

use crate::{CURRENT_VERSION, LATEST_VERSION_URL, ORIGINAL_HEADER, UPDATE_URL};
use crate::error::GenericError;
use crate::message_digest::md5_digest_of_file;
use crate::os::{HOSTS_BACKUP_PATH, HOSTS_PATH};
use crate::parser::parse_hosts_from_file;
use crate::writer::write_hosts_to_file;

pub fn is_installed() -> bool {
    // Maybe there are another condition that can be checked
    is_backed()
}

pub fn remove_temp_file() -> Result<(), std::io::Error> {
    let mut tmp_file = current_dir()?;
    tmp_file.push(".bebasin_tmp");
    if tmp_file.exists() {
        fs::remove_file(tmp_file)?;
    }
    Ok(())
}

pub fn is_backed() -> bool {
    Path::new(HOSTS_BACKUP_PATH).exists()
}

pub fn backup() -> Result<(), GenericError> {
    let hosts = parse_hosts_from_file(HOSTS_PATH)?;
    Ok(
        write_hosts_to_file(
            HOSTS_BACKUP_PATH,
            &hosts,
            ORIGINAL_HEADER,
        )?
    )
}

#[derive(Deserialize, Clone)]
pub struct Checksum {
    linux: String,
    windows: String,
    macos: String,
}

#[derive(Deserialize, Clone)]
pub struct Latest {
    pub version: u64,
    checksum: Checksum,
}

#[derive(Deserialize)]
struct ReleaseAssets {
    name: String,
    size: u32,
    browser_download_url: String,
}

#[derive(Deserialize)]
pub struct Release {
    assets: Vec<ReleaseAssets>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_as_executable<P>(path: &P) -> Result<(), GenericError>
    where
        P: AsRef<Path> + nix::NixPath {
    use std::os::unix::io::IntoRawFd as _;

    // Get file permission
    let permission = nix::sys::stat::stat(path)?.st_mode;
    let mut permission_mode = nix::sys::stat::Mode::from_bits_truncate(permission);
    // Add user executable permission
    permission_mode.insert(nix::sys::stat::Mode::S_IRWXU);

    // Set the file permission
    let file_descriptor = std::fs::File::open(path)?.into_raw_fd();
    Ok(nix::sys::stat::fchmod(file_descriptor, permission_mode)?)
}

pub struct Updater {
    pub latest: Option<Latest>,
}

struct Collector(Vec<u8>);

impl Collector {
    fn new() -> Self {
        Collector(Vec::new())
    }
}

impl curl::easy::Handler for Collector {}

impl Updater {
    pub fn new() -> Updater {
        Updater { latest: None }
    }

    pub fn get_latest_info(&mut self) -> Result<Latest, GenericError> {
        let collector = Collector::new();
        let mut curl_instance = curl::easy::Easy2::new(collector);
        curl_instance.get(true)?;
        curl_instance.url(LATEST_VERSION_URL)?;
        curl_instance.perform()?;

        let mut byte_data = Vec::new();
        let mut curl_instance = curl::easy::Easy::new();
        curl_instance.url(LATEST_VERSION_URL).unwrap();
        {
            let mut handler = curl_instance.transfer();
            handler
                .write_function(|data| {
                    byte_data.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();
            handler.perform().unwrap();
        }
        let string_data = String::from_utf8_lossy(&byte_data);

        self.latest = Some(serde_json::from_str::<Latest>(&string_data)?);

        Ok(self.latest.clone().unwrap())
    }

    pub fn is_updatable(&self) -> bool {
        // Bruh unsafe
        let latest = &self.latest.as_ref().unwrap();

        CURRENT_VERSION < latest.version
    }

    pub fn get_release_data() -> Result<Release, GenericError> {
        let mut byte_data = Vec::new();
        let mut curl_instance = curl::easy::Easy::new();
        curl_instance.url(UPDATE_URL).unwrap();
        curl_instance
            .useragent("User-Agent: Awesome-Octocat-App")
            .unwrap();
        {
            let mut handler = curl_instance.transfer();
            handler
                .write_function(|data| {
                    byte_data.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();
            handler.perform().unwrap();
        }
        let string_data = String::from_utf8_lossy(&byte_data);
        Ok(serde_json::from_str::<Release>(&string_data)?)
    }

    #[cfg(target_os = "windows")]
    fn process_update(&self, release: Release) -> Result<(), ErrorKind> {
        // Bruh unsafe
        let latest = &self.latest.as_ref().unwrap();

        for asset in release.assets {
            if asset.name.contains("windows") {
                let mut byte_data = Vec::new();
                let mut curl_instance = curl::easy::Easy::new();
                curl_instance.url(&asset.browser_download_url).unwrap();
                curl_instance.follow_location(true).unwrap();
                curl_instance.cookie_file("cookie").unwrap();
                curl_instance.cookie_session(true).unwrap();
                {
                    let mut handler = curl_instance.transfer();
                    handler
                        .write_function(|data| {
                            byte_data.extend_from_slice(data);
                            Ok(data.len())
                        })
                        .unwrap();
                    handler.perform().unwrap();
                }

                let mut updated_exe_path = current_dir().unwrap();
                updated_exe_path.push(".bebasin_tmp");
                let mut tmp_exe_path = current_dir().unwrap();
                tmp_exe_path.push(".bebasin_tmp2");
                // Bruh unsafe
                let current_exe_path = &current_exe().unwrap();

                {
                    let mut file_created = fs::File::create(&updated_exe_path).unwrap();
                    file_created.write(byte_data.as_slice());
                }

                match get_md5_digest(&updated_exe_path) {
                    Ok(digest) => {
                        if format!("{:x}", digest) != latest.checksum.windows {
                            return Err(ErrorKind::String(String::from("Download corrupt")));
                        }
                    }
                    Err(err) => return Err(err),
                };

                let mut buf = Vec::new();

                {
                    let zipfile = std::fs::File::open(&updated_exe_path).unwrap();

                    let mut archive = zip::ZipArchive::new(zipfile).unwrap();

                    let mut file = match archive.by_name("bebasin.exe") {
                        Ok(file) => file,
                        Err(err) => return Err(ErrorKind::ZipError(err)),
                    };

                    file.read_to_end(&mut buf);
                }

                std::fs::File::create(&updated_exe_path)
                    .unwrap()
                    .write(&buf);

                if let Err(err) = fs::rename(&current_exe_path, &tmp_exe_path) {
                    return Err(ErrorKind::IOError(err));
                }

                if let Err(err) = fs::rename(&updated_exe_path, &current_exe_path) {
                    return Err(ErrorKind::IOError(err));
                }

                if let Err(err) = fs::rename(&tmp_exe_path, &updated_exe_path) {
                    return Err(ErrorKind::IOError(err));
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn process_update(&self, release: Release) -> Result<(), GenericError> {
        // Bruh unsafe
        let latest = &self.latest.as_ref().unwrap();

        for asset in release.assets {
            if asset.name.contains("linux") {
                let mut byte_data = Vec::new();
                let mut curl_instance = curl::easy::Easy::new();
                println!("{}", asset.browser_download_url);
                curl_instance.url(&asset.browser_download_url).unwrap();
                curl_instance.follow_location(true).unwrap();
                curl_instance.cookie_file("cookie").unwrap();
                curl_instance.cookie_session(true).unwrap();
                {
                    println!("Running");
                    let mut handler = curl_instance.transfer();
                    handler
                        .write_function(|data| {
                            byte_data.extend_from_slice(data);
                            Ok(data.len())
                        })
                        .unwrap();
                    handler.perform().unwrap();
                }

                let mut updated_exe_path = std::env::current_exe().unwrap();
                updated_exe_path.pop();
                updated_exe_path.push(".bebasin_tmp");
                // Bruh unsafe
                let current_exe_path = &std::env::current_exe().unwrap();

                {
                    let mut file_created = fs::File::create(&updated_exe_path).unwrap();
                    file_created.write(byte_data.as_slice())?;
                }

                let digest = md5_digest_of_file(&updated_exe_path)?;

                if format!("{:x}", digest) != latest.checksum.linux {
                    // return Err();
                }

                let mut buf = Vec::new();

                {
                    let zipfile = std::fs::File::open(&updated_exe_path).unwrap();

                    let mut archive = zip::ZipArchive::new(zipfile).unwrap();

                    let mut file = archive.by_name("bebasin")?;

                    file.read_to_end(&mut buf)?;
                }

                std::fs::File::create(&updated_exe_path)
                    .unwrap()
                    .write(&buf)?;

                set_as_executable(&std::path::PathBuf::from(&updated_exe_path))?;

                nix::unistd::unlink(current_exe_path)?;

                fs::rename(&updated_exe_path, current_exe_path)?;
            }
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn process_update(&self, release: Release) -> Result<(), ErrorKind> {
        // Bruh unsafe
        let latest = &self.latest.as_ref().unwrap();

        for asset in release.assets {
            if asset.name.contains("macos") {
                let mut byte_data = Vec::new();
                let mut curl_instance = curl::easy::Easy::new();
                curl_instance.url(&asset.browser_download_url).unwrap();
                curl_instance.follow_location(true).unwrap();
                curl_instance.cookie_file("cookie").unwrap();
                curl_instance.cookie_session(true).unwrap();
                {
                    let mut handler = curl_instance.transfer();
                    handler
                        .write_function(|data| {
                            byte_data.extend_from_slice(data);
                            Ok(data.len())
                        })
                        .unwrap();
                    handler.perform().unwrap();
                }

                let mut updated_exe_path = std::env::current_exe().unwrap();
                updated_exe_path.pop();
                updated_exe_path.push(".bebasin_tmp");
                // Bruh unsafe
                let current_exe_path = &std::env::current_exe().unwrap();

                {
                    let mut file_created = fs::File::create(&updated_exe_path).unwrap();
                    file_created.write(byte_data.as_slice());
                }

                match get_md5_digest(&updated_exe_path) {
                    Ok(digest) => {
                        if format!("{:x}", digest) != latest.checksum.macos {
                            return Err(ErrorKind::String(String::from("Download corrupt")));
                        }
                    }
                    Err(err) => return Err(err),
                };

                let mut buf = Vec::new();

                {
                    let zipfile = std::fs::File::open(&updated_exe_path).unwrap();

                    let mut archive = zip::ZipArchive::new(zipfile).unwrap();

                    let mut file = match archive.by_name("bebasin") {
                        Ok(file) => file,
                        Err(err) => return Err(ErrorKind::ZipError(err)),
                    };

                    file.read_to_end(&mut buf);
                }

                std::fs::File::create(&updated_exe_path)
                    .unwrap()
                    .write(&buf);

                if let Err(err) = set_as_executable(&std::path::PathBuf::from(&updated_exe_path)) {
                    return Err(err);
                }

                if let Err(err) = nix::unistd::unlink(current_exe_path) {
                    return Err(ErrorKind::NixError(err));
                }

                match fs::rename(&updated_exe_path, current_exe_path) {
                    Err(err) => return Err(ErrorKind::IOError(err)),
                    _ => (),
                };
            }
        }
        Ok(())
    }
}
