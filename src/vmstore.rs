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
use crate::statestore::DirectoryStore;

pub struct VMStore {
    store: DirectoryStore,
}

impl VMStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            store: DirectoryStore::new(path)?,
        })
    }

    pub fn path_for_instance(&self, id: &str) -> PathBuf {
        self.store.path().join(id)
    }

    pub fn list_instances(&self) -> Result<Vec<String>, Error> {
        Ok(self.store.list_files()?)
    }

    pub fn new_instance(&mut self, id: &str) -> Result<PathBuf, Error> {
        let path = self.path_for_instance(id);
        std::fs::create_dir(&path)?;
        Ok(path)
    }

    pub fn create_instance_image<P: AsRef<Path>>(
        &mut self,
        id: &str,
        image_path: P,
        resize: Option<u64>,
    ) -> Result<PathBuf, Error> {
        let path = self.path_for_instance(id);

        let imgpath = path.join("instance.qcow2");

        imgutil::create(&imgpath, resize, Some(image_path))?;

        Ok(imgpath)
    }

    pub fn remove_instance(&mut self, id: &str) -> Result<(), Error> {
        let path = self.path_for_instance(id);

        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            std::fs::remove_file(entry.path())?;
        }

        std::fs::remove_dir(&path)?;

        Ok(())
    }
}

mod imgutil {
    use std::path::Path;
    use std::process::Command;

    use tracing::debug;

    use crate::error::Error;

    pub fn create<P: AsRef<Path>, B: AsRef<Path>>(
        filepath: P,
        resize: Option<u64>,
        backing_file: Option<B>,
    ) -> Result<(), Error> {
        let mut cmd = Command::new("/usr/bin/qemu-img");
        cmd.arg("create");
        cmd.arg("-q");

        if let Some(bf) = backing_file {
            cmd.arg("-b");
            cmd.arg(bf.as_ref());
        }

        cmd.arg("-f");
        cmd.arg("qcow2");
        cmd.arg(filepath.as_ref());

        if let Some(size) = resize {
            cmd.arg(size.to_string());
        }

        debug!("Running: {:?}", cmd);
        let r = cmd.status()?;
        if r.success() {
            return Ok(());
        } else {
            return Err("failed to create new image".into());
        }
    }
}
