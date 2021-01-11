use crate::error::ErrorKind;
use crate::os::{HOSTS_BACKUP_PATH, HOSTS_PATH};
use crate::parser::{parse_from_file, write_to_file};
use crate::{CURRENT_VERSION, LATEST_VERSION_URL, UPDATE_URL, HOSTS_HEADER, DEFAULT_HOSTS};
use serde::Deserialize;
use std::env::{current_dir, current_exe};
use std::fs;
use std::io::{self, Read};
use std::io::Write as _;
use std::path::Path;

pub fn is_installed() -> bool {
    // Maybe there are another condition that can be checked
    is_backed()
}

pub fn hosts_exists() -> bool {
    Path::new(HOSTS_PATH).exists()
}

pub fn create_default_hosts() -> io::Result<()> {
    let mut file = fs::File::create(HOSTS_PATH)?;
    file.write_all(format!("{}\n{}", HOSTS_HEADER, DEFAULT_HOSTS).as_bytes())?;
    Ok(())
}

pub fn remove_temp_file() {
    let mut tmp_file = current_dir().unwrap();
    tmp_file.push(".bebasin_tmp");
    if tmp_file.exists() {
        fs::remove_file(tmp_file);
    }
}

pub fn is_backed() -> bool {
    Path::new(HOSTS_BACKUP_PATH).exists()
}

pub fn backup() -> Result<(), ErrorKind> {
    match parse_from_file(HOSTS_PATH) {
        Ok(hosts_local) => match write_to_file(
            HOSTS_BACKUP_PATH,
            &hosts_local,
            include_str!("../misc/header-backup"),
        ) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
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
struct Release {
    assets: Vec<ReleaseAssets>,
}

fn md5_digest<R: std::io::Read>(mut reader: R) -> Result<md5::Digest, std::io::Error> {
    let mut context = md5::Context::new();
    let mut buffer = [0; 1024];

    loop {
        let count = match reader.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    break;
                } else {
                    size
                }
            }
            Err(err) => return Err(err),
        };
        context.consume(&buffer[..count]);
    }

    Ok(context.compute())
}

fn get_md5_digest<P: AsRef<Path>>(path: &P) -> Result<md5::Digest, ErrorKind> {
    match std::fs::File::open(path) {
        Ok(file) => match md5_digest(file) {
            Ok(x) => Ok(x),
            Err(err) => Err(ErrorKind::IOError(err)),
        },
        Err(err) => Err(ErrorKind::IOError(err)),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_as_executable<P: AsRef<Path> + nix::NixPath>(path: &P) -> Result<(), ErrorKind> {
    use std::os::unix::io::IntoRawFd as _;

    // Get file permission
    let permission = match nix::sys::stat::stat(path) {
        Ok(stat) => stat.st_mode,
        Err(err) => return Err(ErrorKind::NixError(err)),
    };
    let mut permission_mode = nix::sys::stat::Mode::from_bits_truncate(permission);
    // Add user executable permission
    permission_mode.insert(nix::sys::stat::Mode::S_IRWXU);

    // Set the file permission
    let file_descriptor = match std::fs::File::open(path) {
        Ok(file) => file.into_raw_fd(),
        Err(err) => return Err(ErrorKind::IOError(err)),
    };
    match nix::sys::stat::fchmod(file_descriptor, permission_mode) {
        Ok(_) => Ok(()),
        Err(err) => Err(ErrorKind::NixError(err)),
    }
}

pub struct Updater {
    pub latest: Option<Latest>,
}

impl Updater {
    pub fn new() -> Updater {
        Updater { latest: None }
    }

    pub fn get_latest_info(&mut self) -> Result<Latest, ErrorKind> {
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

        self.latest = match serde_json::from_str::<Latest>(&string_data) {
            Ok(latest_data) => Some(latest_data),
            Err(err) => return Err(ErrorKind::SerdeJSONError(err)),
        };

        Ok(self.latest.clone().unwrap())
    }

    pub fn is_updatable(&self) -> bool {
        // Bruh unsafe
        let latest = &self.latest.as_ref().unwrap();

        CURRENT_VERSION < latest.version
    }

    pub fn update(&self) -> Result<(), ErrorKind> {
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
        let release_data = match serde_json::from_str::<Release>(&string_data) {
            Ok(release_data) => release_data,
            Err(err) => return Err(ErrorKind::SerdeJSONError(err)),
        };

        self.process_update(release_data)
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
    fn process_update(&self, release: Release) -> Result<(), ErrorKind> {
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
                    file_created.write(byte_data.as_slice());
                }

                match get_md5_digest(&updated_exe_path) {
                    Ok(digest) => {
                        if format!("{:x}", digest) != latest.checksum.linux {
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

                if let Err(err) = fs::rename(&updated_exe_path, current_exe_path) {
                    return Err(ErrorKind::IOError(err));
                }
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
