//! General file information, similar to `stat(1)`.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

use bytesize::ByteSize;

/// Shown for fields the platform cannot provide.
const UNAVAILABLE: &str = "n/a";

/// General file information.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Name of the file.
    pub name: String,
    /// Size of the file, human readable.
    pub size: String,
    /// Number of blocks allocated for the file.
    pub blocks: String,
    /// Block size.
    pub block_size: String,
    /// Device ID.
    pub device: String,
    /// Inode number.
    pub inode: String,
    /// Number of hard links.
    pub links: String,
    /// Access mode, e.g. `0755/rwxr-xr-x`.
    pub mode: String,
    /// Owner, e.g. `1000/rafael`.
    pub uid: String,
    /// Group, e.g. `1000/rafael`.
    pub gid: String,
    /// Access date.
    pub accessed: String,
    /// Modify date.
    pub modified: String,
    /// Change date.
    pub changed: String,
    /// Birth date.
    pub created: String,
}

impl FileInfo {
    /// Information for a file known only by name and contents (browser uploads).
    pub fn from_bytes(name: &str, len: usize) -> Self {
        let na = || UNAVAILABLE.to_string();
        Self {
            name: name.to_string(),
            size: ByteSize(len as u64).to_string(),
            blocks: na(),
            block_size: na(),
            device: na(),
            inode: na(),
            links: na(),
            mode: na(),
            uid: na(),
            gid: na(),
            accessed: na(),
            modified: na(),
            changed: na(),
            created: na(),
        }
    }

    /// The rows shown in the file information panel.
    pub fn rows(&self) -> [(&'static str, &str); 13] {
        [
            ("Size", &self.size),
            ("Blocks", &self.blocks),
            ("Block size", &self.block_size),
            ("Device", &self.device),
            ("Inode", &self.inode),
            ("Links", &self.links),
            ("Access", &self.mode),
            ("Uid", &self.uid),
            ("Gid", &self.gid),
            ("Access", &self.accessed),
            ("Modify", &self.modified),
            ("Change", &self.changed),
            ("Birth", &self.created),
        ]
    }
}

/// Renders the permission bits as `rwxr-xr-x`, with `s`/`S`/`t`/`T` for
/// setuid, setgid and sticky like `stat(1)`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn mode_string(mode: u32) -> String {
    let bits = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    let mut s: Vec<char> = bits
        .iter()
        .zip("rwxrwxrwx".chars())
        .map(|(bit, c)| if mode & bit != 0 { c } else { '-' })
        .collect();
    for (special, index, set, unset) in [
        (0o4000, 2, 's', 'S'),
        (0o2000, 5, 's', 'S'),
        (0o1000, 8, 't', 'T'),
    ] {
        if mode & special != 0 {
            s[index] = if s[index] == 'x' { set } else { unset };
        }
    }
    s.into_iter().collect()
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{FileInfo, mode_string};
    use crate::error::Result;
    use bytesize::ByteSize;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::Path,
        sync::OnceLock,
    };
    use sysinfo::{Gid, Groups, Uid, Users};

    /// User and group tables, read once per process.
    fn users() -> &'static Users {
        static USERS: OnceLock<Users> = OnceLock::new();
        USERS.get_or_init(Users::new_with_refreshed_list)
    }

    fn groups() -> &'static Groups {
        static GROUPS: OnceLock<Groups> = OnceLock::new();
        GROUPS.get_or_init(Groups::new_with_refreshed_list)
    }

    /// Formats a local timestamp, or "not supported" when the filesystem has none.
    fn format_time(time: Option<chrono::DateTime<chrono::Local>>) -> String {
        time.map_or_else(
            || String::from("not supported"),
            |t| t.format("%Y-%m-%d %H:%M:%S.%f %z").to_string(),
        )
    }

    impl FileInfo {
        /// Reads the metadata of the file at `path`.
        pub fn new(path: &Path) -> Result<Self> {
            let metadata = fs::metadata(path)?;
            let mode = metadata.permissions().mode();
            let gid = Gid::try_from(metadata.gid() as usize).ok();
            let user = Uid::try_from(metadata.uid() as usize)
                .ok()
                .and_then(|uid| users().get_user_by_id(&uid))
                .map(|v| v.name());
            let group = groups()
                .list()
                .iter()
                .find(|g| gid.as_ref() == Some(g.id()))
                .map(|v| v.name());
            Ok(Self {
                name: path
                    .file_name()
                    .map(|v| v.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                size: ByteSize(metadata.len()).to_string(),
                blocks: metadata.blocks().to_string(),
                block_size: metadata.blksize().to_string(),
                device: metadata.dev().to_string(),
                inode: metadata.ino().to_string(),
                links: metadata.nlink().to_string(),
                mode: format!("{:04o}/{}", mode & 0o7777, mode_string(mode)),
                uid: format!("{}/{}", metadata.uid(), user.unwrap_or("?")),
                gid: format!("{}/{}", metadata.gid(), group.unwrap_or("?")),
                // Timestamps are informational; never fail the analysis over one.
                accessed: format_time(metadata.accessed().ok().map(Into::into)),
                modified: format_time(metadata.modified().ok().map(Into::into)),
                changed: format_time(
                    chrono::DateTime::from_timestamp(
                        metadata.ctime(),
                        metadata.ctime_nsec() as u32,
                    )
                    .map(|t| t.with_timezone(&chrono::Local)),
                ),
                created: format_time(metadata.created().ok().map(Into::into)),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_bits() {
        assert_eq!(mode_string(0o755), "rwxr-xr-x");
        assert_eq!(mode_string(0o600), "rw-------");
        assert_eq!(mode_string(0o000), "---------");
        assert_eq!(mode_string(0o4755), "rwsr-xr-x");
        assert_eq!(mode_string(0o6644), "rwSr-Sr--");
        assert_eq!(mode_string(0o1777), "rwxrwxrwt");
        assert_eq!(mode_string(0o1776), "rwxrwxrwT");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn stat_bin_ls() {
        let info = FileInfo::new(std::path::Path::new("/bin/ls")).expect("stat /bin/ls");
        assert_eq!(info.name, "ls");
        assert!(info.mode.contains('x'));
        assert!(!info.modified.is_empty());
    }

    #[test]
    fn uploaded_file_has_size_only() {
        let info = FileInfo::from_bytes("sample", 2048);
        assert_eq!(info.size, "2.0 KiB");
        assert_eq!(info.inode, UNAVAILABLE);
    }
}
