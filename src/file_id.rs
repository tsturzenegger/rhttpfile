use core::str;
use rocket::UriDisplayPath;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{env, fs};

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use rand::{self, Rng};
use rocket::request::FromParam;

/// A _probably_ unique file ID.
#[derive(UriDisplayPath)]
pub struct FileId<'a>(Cow<'a, str>);

impl FileId<'_> {
    /// Generate a _probably_ unique ID with `size` characters. For readability,
    /// the characters used are from the sets [0-9], [A-Z], [a-z]. The
    /// probability of a collision depends on the value of `size` and the number
    /// of IDs generated thus far.
    pub fn new(file_name: &str, size: usize) -> FileId<'static> {
        const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

        let mut id = String::with_capacity(size);
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            id.push(BASE62[rng.gen::<usize>() % 62] as char);
        }
        let mut file_name = URL_SAFE.encode(file_name);
        file_name.push_str(&id);

        FileId(Cow::Owned(file_name))
    }

    /// Returns the path to the file in `upload/` corresponding to this ID.
    /// Panics if no upload folder could be created/used.
    pub fn file_path(&self) -> PathBuf {
        let mut current_exe = env::current_exe().expect("could not access current dir.");
        current_exe.pop();
        let upload_dir = current_exe.join("upload");
        fs::create_dir_all(&upload_dir).expect("could not create upload dir.");
        Path::new(&upload_dir).join(self.0.as_ref())
    }
}

/// Returns an instance of `FileId` if the path segment is a valid ID.
/// Otherwise returns the invalid ID as the `Err` value.
impl<'a> FromParam<'a> for FileId<'a> {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        param
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '=')
            .then(|| FileId(param.into()))
            .ok_or(param)
    }
}
