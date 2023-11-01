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

use std::path::{Path, PathBuf};

use crate::error::Error;

pub struct DirectoryStore {
    path: PathBuf,
}

impl DirectoryStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        if !path.as_ref().is_dir() {
            std::fs::create_dir_all(path.as_ref())?;
        }

        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn list_files(&self) -> Result<Vec<String>, Error> {
        let entries = std::fs::read_dir(&self.path)?
            .map(|res| res.map(|e| e.file_name()))
            .collect::<Result<Vec<_>, std::io::Error>>()?;

        let mut str_entries = Vec::new();

        for e in entries {
            if let Ok(s) = e.into_string() {
                str_entries.push(s);
            }
        }

        Ok(str_entries)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_list() {
        let d = DirectoryStore::new(".").unwrap();

        let files = d.list_files().unwrap();

        eprintln!("{:?}", files);
        assert!(files.contains(&"src".to_string()));
    }
}
