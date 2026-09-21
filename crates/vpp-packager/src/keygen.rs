//! `vpppack keygen`: a new publisher key pair.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vpp_format::SigningKey;

/// Creates `<name>.key` (secret) and `<name>.pub`, refusing to overwrite an existing secret key.
pub(crate) fn keygen(name: &Path, out: &mut dyn Write) -> Result<()> {
    let secret_path = with_extension(name, "key");
    let public_path = with_extension(name, "pub");
    let key = SigningKey::generate()?;

    // create_new fails if the file exists, with no gap between checking and writing.
    let mut secret = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&secret_path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => bail!(
            "{} already exists; refusing to overwrite a secret key",
            secret_path.display()
        ),
        Err(e) => {
            return Err(e).with_context(|| format!("cannot create {}", secret_path.display()));
        }
    };
    secret
        .write_all(key.to_file_text().as_bytes())
        .with_context(|| format!("cannot write {}", secret_path.display()))?;
    fs::write(&public_path, key.public_key().to_file_text())
        .with_context(|| format!("cannot write {}", public_path.display()))?;

    writeln!(out, "vpppack: new publisher key")?;
    writeln!(
        out,
        "  secret  {}   (keep private, never commit)",
        secret_path.display()
    )?;
    writeln!(out, "  public  {}", public_path.display())?;
    writeln!(out, "  key id  {}", key.public_key())?;
    Ok(())
}

/// `name` plus `.ext`, keeping any dots already in the name (`site.v2` → `site.v2.key`).
fn with_extension(name: &Path, ext: &str) -> PathBuf {
    let mut path = name.as_os_str().to_owned();
    path.push(".");
    path.push(ext);
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;
    use vpp_format::PublicKey;

    #[test]
    fn writes_a_matching_key_pair() {
        let dir = TestDir::new("keygen");
        let mut out = Vec::new();
        keygen(&dir.path().join("site"), &mut out).unwrap();

        let secret = fs::read_to_string(dir.path().join("site.key")).unwrap();
        let public = fs::read_to_string(dir.path().join("site.pub")).unwrap();
        let key = SigningKey::from_file_text(&secret).unwrap();
        assert_eq!(
            PublicKey::from_file_text(&public).unwrap(),
            key.public_key()
        );
        assert!(
            String::from_utf8(out)
                .unwrap()
                .contains(&key.public_key().to_string())
        );
    }

    #[test]
    fn never_overwrites_a_secret_key() {
        let dir = TestDir::new("keygen-twice");
        let name = dir.path().join("publisher");
        keygen(&name, &mut Vec::new()).unwrap();
        let first = fs::read(dir.path().join("publisher.key")).unwrap();

        let err = keygen(&name, &mut Vec::new()).unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
        assert_eq!(fs::read(dir.path().join("publisher.key")).unwrap(), first);
    }

    #[test]
    fn keeps_dots_in_the_name() {
        assert_eq!(
            with_extension(Path::new("site.v2"), "key"),
            PathBuf::from("site.v2.key")
        );
    }
}
