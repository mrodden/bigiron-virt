//  Copyright (C) 2023 IBM Corp.
//
//  This library is free software; you can redistribute it and/or
//  modify it under the terms of the GNU Lesser General Public
//  License as published by the Free Software Foundation; either
//  version 2.1 of the License, or (at your option) any later version.
//
//  This library is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
//  Lesser General Public License for more details.
//
//  You should have received a copy of the GNU Lesser General Public
//  License along with this library; if not, write to the Free Software
//  Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301
//  USA

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use hex;
use sha2::{Digest, Sha256};
use tracing::info;
use url::Url;

use crate::error::Error;
use crate::statestore::DirectoryStore;

// image repo based on a local directory
pub struct Directory {
    store: DirectoryStore,
}

pub type ImageId = String;

impl Directory {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            store: DirectoryStore::new(path)?,
        })
    }

    pub fn images(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .store
            .list_files()?
            .into_iter()
            .filter(|f| f.ends_with(".qcow2"))
            .collect())
    }

    pub fn add_image(&mut self, url: &Url, hash: &str) -> Result<ImageId, Error> {
        match url.scheme() {
            "file" => {}
            _ => return Err(format!("Url scheme not supported: {:?}", url.scheme()).into()),
        };

        let to_path = self.store.path().join(format!("{}.qcow2", hash));
        if to_path.exists() {
            return Ok(hash.to_string());
        }

        let from_path = url
            .to_file_path()
            .expect("error converting URL to filepath");

        let mut image_stream = std::fs::File::open(&from_path)?;

        let mut out_stream = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&to_path)?;

        let mut h = Sha256::new();

        info!("Copying new image into image repo at {:?}", to_path);

        // copy image to repo, while hashing
        let mut buf = [0; 128 * 1024];
        let mut n = image_stream.read(&mut buf)?;

        while n > 0 {
            h.write_all(&buf[..n])?;
            out_stream.write_all(&buf[..n])?;
            n = image_stream.read(&mut buf)?;
        }

        let r = h.finalize();
        let hx = hex::encode(r);

        // check hash against given hash
        if hx != hash {
            // remove non-matching file
            std::fs::remove_file(&to_path).expect("error while removing invalid image file");
            return Err(String::from("Given hash value does not match image data hash").into());
        } else {
            info!("New image hash='{}' matches given hash", hx);
        }

        Ok(hash.to_string())
    }

    pub fn get_image(&self, id: &ImageId) -> Result<PathBuf, Error> {
        let path = self.store.path().join(format!("{}.qcow2", id));

        if !path.is_file() {
            return Err(String::from(format!("No image with id='{}' found", id)).into());
        }

        Ok(path)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_list() {
        let d = Directory::new("./").unwrap();
        let images = d.images().unwrap();

        eprintln!("{:?}", images);
        assert!(!images.contains(&"src".to_string()));
    }
}
