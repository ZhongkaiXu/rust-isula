use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Seek};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Component, Path, PathBuf};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn absolute(path: &str) -> io::Result<PathBuf> {
    if path.is_empty() {
        return Err(invalid("path cannot be empty"));
    }
    let path = std::path::absolute(path)?;
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                clean.pop();
            }
            Component::CurDir => {}
            _ => clean.push(part),
        }
    }
    Ok(clean)
}

fn append(
    builder: &mut tar::Builder<&mut File>,
    source: &Path,
    name: &Path,
    links: &mut HashMap<(u64, u64), PathBuf>,
) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    let attributes = read_attributes(source)?;
    builder.append_pax_extensions(
        attributes
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_slice())),
    )?;
    if metadata.is_file() && metadata.nlink() > 1 {
        let key = (metadata.dev(), metadata.ino());
        if let Some(first) = links.get(&key) {
            let mut header = tar::Header::new_gnu();
            header.set_metadata(&metadata);
            header.set_entry_type(tar::EntryType::Link);
            header.set_size(0);
            return builder.append_link(&mut header, name, first);
        }
        links.insert(key, name.into());
    }
    let file_type = metadata.file_type();
    if file_type.is_fifo() || file_type.is_char_device() || file_type.is_block_device() {
        let mut header = tar::Header::new_gnu();
        header.set_metadata(&metadata);
        header.set_size(0);
        header.set_entry_type(if file_type.is_fifo() {
            tar::EntryType::Fifo
        } else if file_type.is_char_device() {
            tar::EntryType::Char
        } else {
            tar::EntryType::Block
        });
        header.set_device_major(libc::major(metadata.rdev()))?;
        header.set_device_minor(libc::minor(metadata.rdev()))?;
        return builder.append_data(&mut header, name, io::empty());
    }
    builder.append_path_with_name(source, name)?;
    if metadata.is_dir() {
        let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            append(builder, &entry.path(), &name.join(entry.file_name()), links)?;
        }
    }
    Ok(())
}

fn read_attributes(path: &Path) -> io::Result<Vec<(String, Vec<u8>)>> {
    let names = match xattr::list(path) {
        Ok(names) => names,
        Err(err) if err.raw_os_error() == Some(libc::ENOTSUP) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut attributes = Vec::new();
    for name in names {
        if let Some(value) = xattr::get(path, &name)? {
            let key = name
                .to_str()
                .ok_or_else(|| invalid("invalid extended attribute name"))?;
            attributes.push((format!("SCHILY.xattr.{key}"), value));
        }
    }
    Ok(attributes)
}

pub(super) fn pack(source: &str) -> io::Result<(File, String, bool)> {
    let path = absolute(source)?;
    let contents = source == "." || source.ends_with("/.") || path == Path::new("/");
    let (path, name) = if contents {
        (fs::canonicalize(&path)?, PathBuf::from("."))
    } else {
        let parent = fs::canonicalize(
            path.parent()
                .ok_or_else(|| invalid("invalid source path"))?,
        )?;
        let name = path
            .file_name()
            .ok_or_else(|| invalid("invalid source path"))?;
        (parent.join(name), PathBuf::from(name))
    };
    let archive_path = if source.ends_with('/') {
        fs::canonicalize(&path)?
    } else {
        path.clone()
    };
    let is_dir = fs::symlink_metadata(&archive_path)?.is_dir();
    if (contents || source.ends_with('/')) && !is_dir {
        return Err(invalid("source is not a directory"));
    }
    let mut file = tempfile::tempfile()?;
    {
        let mut builder = tar::Builder::new(&mut file);
        builder.follow_symlinks(false);
        append(&mut builder, &archive_path, &name, &mut HashMap::new())?;
        builder.finish()?;
    }
    file.rewind()?;
    let mut request_path = path.to_string_lossy().into_owned();
    if contents && request_path != "/" {
        request_path.push_str("/.");
    } else if source.ends_with('/') && !request_path.ends_with('/') {
        request_path.push('/');
    }
    Ok((file, request_path, is_dir))
}

fn safe_path(path: &Path) -> io::Result<PathBuf> {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            _ => return Err(invalid("archive path escapes destination")),
        }
    }
    Ok(result)
}

fn rebase(path: &Path, source: &Path, target: Option<&Path>) -> io::Result<PathBuf> {
    let path = safe_path(path)?;
    match target {
        None => Ok(if path.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            path
        }),
        Some(target) => {
            let suffix = path
                .strip_prefix(source)
                .map_err(|_| invalid("archive entry does not match source"))?;
            Ok(if suffix.as_os_str().is_empty() {
                target.to_path_buf()
            } else {
                target.join(suffix)
            })
        }
    }
}

// Resolve destination symlinks as the native cp does, including a dangling leaf.
fn destination(path: PathBuf) -> io::Result<PathBuf> {
    let mut path = path;
    for _ in 0..11 {
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let link = fs::read_link(&path)?;
                path = if link.is_absolute() {
                    link
                } else {
                    path.parent().unwrap().join(link)
                };
            }
            Ok(_) => return Ok(path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(path),
            Err(err) => return Err(err),
        }
    }
    Err(invalid("too many destination symlinks"))
}

pub(super) fn unpack(mut file: File, source: &str, is_dir: bool, dest: &str) -> io::Result<()> {
    let dest_path = destination(absolute(dest)?)?;
    let metadata = match fs::metadata(&dest_path) {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    let dest_dir = metadata.as_ref().is_some_and(|meta| meta.is_dir());
    if metadata.is_some() && !dest_dir && (dest.ends_with('/') || dest.ends_with("/.")) {
        return Err(invalid("destination is not a directory"));
    }
    if metadata.is_some() && !dest_dir && is_dir {
        return Err(invalid("cannot copy directory to file"));
    }
    if metadata.is_none() && !is_dir && (dest.ends_with('/') || dest.ends_with("/.")) {
        return Err(invalid("no such directory"));
    }
    let (directory, rename) = if dest_dir {
        (fs::canonicalize(&dest_path)?, None)
    } else {
        let parent = dest_path
            .parent()
            .ok_or_else(|| invalid("invalid destination"))?;
        (
            fs::canonicalize(parent)?,
            Some(PathBuf::from(
                dest_path
                    .file_name()
                    .ok_or_else(|| invalid("invalid destination"))?,
            )),
        )
    };
    let src = source.trim_end_matches('/');
    let base = if src.is_empty() || src.ends_with("/.") || src == "." {
        PathBuf::new()
    } else {
        PathBuf::from(
            Path::new(src)
                .file_name()
                .ok_or_else(|| invalid("invalid source"))?,
        )
    };
    // Validate and rebase before writing any destination files. Temporary archives
    // bound memory use and keep failed RPCs from leaving partially extracted data.
    let mut rebased = tempfile::tempfile()?;
    let mut count = 0;
    let mut special = HashMap::new();
    {
        let mut builder = tar::Builder::new(&mut rebased);
        let mut archive = tar::Archive::new(&mut file);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = rebase(&entry.path()?, &base, rename.as_deref())?;
            let mut attributes = Vec::new();
            if let Some(extensions) = entry.pax_extensions()? {
                for extension in extensions {
                    let extension = extension?;
                    let key = extension
                        .key()
                        .map_err(|_| invalid("invalid archive attribute name"))?;
                    if key.starts_with("SCHILY.xattr.") {
                        attributes.push((key.to_owned(), extension.value_bytes().to_vec()));
                    }
                }
            }
            builder.append_pax_extensions(
                attributes
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_slice())),
            )?;
            let mut header = entry.header().clone();
            let kind = header.entry_type();
            if kind.is_fifo() || kind.is_character_special() || kind.is_block_special() {
                special.insert(path.clone(), header.clone());
            } else {
                special.remove(&path);
            }
            if header.entry_type().is_hard_link() || header.entry_type().is_symlink() {
                let link = entry
                    .link_name()?
                    .ok_or_else(|| invalid("missing archive link target"))?
                    .into_owned();
                let link = if header.entry_type().is_hard_link() {
                    rebase(&link, &base, rename.as_deref())?
                } else {
                    link
                };
                builder.append_link(&mut header, path, link)?;
            } else {
                builder.append_data(&mut header, path, &mut entry)?;
            }
            count += 1;
        }
        if count == 0 {
            return Err(invalid("empty copy archive"));
        }
        builder.finish()?;
    }
    rebased.rewind()?;
    let mut archive = tar::Archive::new(rebased);
    archive.set_preserve_permissions(true);
    archive.set_unpack_xattrs(true);
    // Ownership can only be restored by a privileged client.
    archive.set_preserve_ownerships(unsafe { libc::geteuid() } == 0);
    archive.unpack(&directory)?;
    for (path, header) in special {
        restore_special(&directory, &path, &header)?;
    }
    Ok(())
}

fn restore_special(directory: &Path, path: &Path, header: &tar::Header) -> io::Result<()> {
    use std::ffi::CString;
    let target = directory.join(path);
    let parent = fs::canonicalize(
        target
            .parent()
            .ok_or_else(|| invalid("invalid special file path"))?,
    )?;
    if !parent.starts_with(directory) {
        return Err(invalid("special file escapes destination"));
    }
    let target = parent.join(
        target
            .file_name()
            .ok_or_else(|| invalid("invalid special file name"))?,
    );
    // tar's extractor writes unrecognized node types as regular files. Replace
    // that safe placeholder with the actual FIFO/device after extracting parents.
    fs::remove_file(&target)?;
    let name = CString::new(target.as_os_str().as_encoded_bytes())
        .map_err(|_| invalid("invalid file name"))?;
    let kind = header.entry_type();
    let mode = header.mode()?;
    let file_type = if kind.is_fifo() {
        libc::S_IFIFO
    } else if kind.is_character_special() {
        libc::S_IFCHR
    } else {
        libc::S_IFBLK
    };
    let device = if kind.is_fifo() {
        0
    } else {
        libc::makedev(
            header.device_major()?.unwrap_or(0),
            header.device_minor()?.unwrap_or(0),
        )
    };
    // SAFETY: the path is NUL-terminated; mode and device come from the tar header.
    if unsafe { libc::mknod(name.as_ptr(), file_type | mode, device) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::geteuid() } == 0 {
        let uid = u32::try_from(header.uid()?).map_err(|_| invalid("invalid file UID"))?;
        let gid = u32::try_from(header.gid()?).map_err(|_| invalid("invalid file GID"))?;
        if unsafe { libc::chown(name.as_ptr(), uid, gid) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    if unsafe { libc::chmod(name.as_ptr(), mode) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let stamp = libc::timespec {
        tv_sec: header
            .mtime()?
            .try_into()
            .map_err(|_| invalid("invalid file timestamp"))?,
        tv_nsec: 0,
    };
    let times = [stamp, stamp];
    if unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            name.as_ptr(),
            times.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn roundtrip_directory_preserves_links_modes_and_rebases_hardlinks() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::write(source.join("file"), b"hello\0\xff").unwrap();
        fs::set_permissions(source.join("file"), fs::Permissions::from_mode(0o751)).unwrap();
        xattr::set(source.join("file"), "user.risula", b"attribute").unwrap();
        fs::hard_link(source.join("file"), source.join("hard")).unwrap();
        symlink("file", source.join("link")).unwrap();
        let (file, path, is_dir) = pack(source.to_str().unwrap()).unwrap();
        let destination = root.path().join("renamed");
        unpack(file, &path, is_dir, destination.to_str().unwrap()).unwrap();
        assert_eq!(fs::read(destination.join("file")).unwrap(), b"hello\0\xff");
        assert_eq!(
            fs::metadata(destination.join("file")).unwrap().mode() & 0o777,
            0o751
        );
        assert_eq!(
            fs::metadata(destination.join("file")).unwrap().ino(),
            fs::metadata(destination.join("hard")).unwrap().ino()
        );
        assert_eq!(
            fs::read_link(destination.join("link")).unwrap(),
            Path::new("file")
        );
        assert!(destination.join("empty").is_dir());
        assert_eq!(
            xattr::get(destination.join("file"), "user.risula").unwrap(),
            Some(b"attribute".to_vec())
        );
    }

    #[test]
    fn directory_contents_and_destination_symlink() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/file"), b"content").unwrap();
        fs::create_dir(root.path().join("dst")).unwrap();
        symlink("dst", root.path().join("link")).unwrap();
        let source = format!("{}/src/.", root.path().display());
        let (file, path, is_dir) = pack(&source).unwrap();
        unpack(
            file,
            &path,
            is_dir,
            root.path().join("link").to_str().unwrap(),
        )
        .unwrap();
        assert_eq!(fs::read(root.path().join("dst/file")).unwrap(), b"content");
    }

    #[test]
    fn fifo_is_not_silently_converted_to_regular_file() {
        use std::os::unix::fs::FileTypeExt;
        let root = tempfile::tempdir().unwrap();
        let mut file = tempfile::tempfile().unwrap();
        {
            let mut builder = tar::Builder::new(&mut file);
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Fifo);
            header.set_size(0);
            header.set_mode(0o640);
            header.set_uid(unsafe { libc::geteuid() }.into());
            header.set_gid(unsafe { libc::getegid() }.into());
            header.set_mtime(1234);
            builder
                .append_data(&mut header, "pipe", io::empty())
                .unwrap();
            builder.finish().unwrap();
        }
        file.rewind().unwrap();
        unpack(file, "/pipe", false, root.path().to_str().unwrap()).unwrap();
        let meta = fs::symlink_metadata(root.path().join("pipe")).unwrap();
        assert!(meta.file_type().is_fifo());
        assert_eq!(meta.mode() & 0o777, 0o640);
        assert_eq!(meta.mtime(), 1234);
        let (file, source, is_dir) = pack(root.path().join("pipe").to_str().unwrap()).unwrap();
        unpack(
            file,
            &source,
            is_dir,
            root.path().join("copied").to_str().unwrap(),
        )
        .unwrap();
        assert!(fs::symlink_metadata(root.path().join("copied"))
            .unwrap()
            .file_type()
            .is_fifo());
    }

    #[test]
    fn rejects_missing_parent_and_directory_over_file() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), b"keep").unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        for dest in [root.path().join("file"), root.path().join("missing/new")] {
            let (file, source, is_dir) = pack(root.path().join("src").to_str().unwrap()).unwrap();
            assert!(unpack(file, &source, is_dir, dest.to_str().unwrap()).is_err());
        }
        assert_eq!(fs::read(root.path().join("file")).unwrap(), b"keep");
    }

    #[test]
    fn rejects_traversal_and_symlink_parent_escape() {
        let root = tempfile::tempdir().unwrap();
        let dest = root.path().join("dest");
        fs::create_dir(&dest).unwrap();
        for traversal in [true, false] {
            let mut file = tempfile::tempfile().unwrap();
            {
                let mut builder = tar::Builder::new(&mut file);
                if !traversal {
                    let mut header = tar::Header::new_gnu();
                    header.set_mode(0o777);
                    header.set_size(0);
                    header.set_entry_type(tar::EntryType::Symlink);
                    builder
                        .append_link(&mut header, "link", root.path())
                        .unwrap();
                }
                let mut header = tar::Header::new_gnu();
                header.set_size(4);
                header.set_mode(0o644);
                let name = if traversal {
                    b"../escape".as_slice()
                } else {
                    b"link/escape".as_slice()
                };
                header.as_mut_bytes()[..name.len()].copy_from_slice(name);
                header.set_cksum();
                builder.append(&header, b"evil".as_slice()).unwrap();
                builder.finish().unwrap();
            }
            file.rewind().unwrap();
            assert!(unpack(file, "/.", true, dest.to_str().unwrap()).is_err());
            assert!(!root.path().join("escape").exists());
        }
    }
}
