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
use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::Error;

pub fn create_iso<P, Q, R, N>(
    output_path: P,
    user_data: Q,
    meta_data: R,
    network_data: &Option<N>,
) -> Result<(), Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
    N: AsRef<Path>,
{
    let isoprog: &str = "/usr/bin/mkisofs";

    let mut cmd = Command::new(&isoprog);

    cmd.arg("-output")
        .arg(output_path.as_ref().to_str().unwrap())
        .arg("-input-charset")
        .arg("utf-8")
        .arg("-volid")
        .arg("cidata")
        .arg("-joliet")
        .arg("-r")
        .arg(user_data.as_ref().to_str().unwrap())
        .arg(meta_data.as_ref().to_str().unwrap());

    if let Some(nd) = network_data {
        cmd.arg(nd.as_ref().to_str().unwrap());
    }

    let output = cmd.output().expect("error executing mkisofs/genisoimage");

    debug!("mkisofs output: {:?}", output);

    if !output.status.success() {
        return Err(format!("{:?}", output).into());
    }

    Ok(())
}

pub struct Builder {
    metadata: Metadata,
    userdata: Option<Vec<u8>>,
    network_config: Option<Vec<u8>>,
}

impl Builder {
    pub fn new(instance_name: &str) -> Self {
        let md = Metadata::new(instance_name);

        Self {
            metadata: md,
            userdata: None,
            network_config: None,
        }
    }

    pub fn metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    pub fn add_userdata(&mut self, userdata: Vec<u8>) -> &mut Self {
        self.userdata = Some(userdata);
        self
    }

    pub fn add_network_config(&mut self, network_config: Vec<u8>) -> &mut Self {
        self.network_config = Some(network_config);
        self
    }

    pub fn build<P: AsRef<Path>>(&mut self, base_dir: P) -> Result<PathBuf, Error> {
        let cd_dir = base_dir.as_ref().join("cidata-dir");

        std::fs::create_dir_all(&cd_dir)?;

        // create iso outside data directory, since we will be cleaning up the data dir
        let iso_path = base_dir.as_ref().join("cidata.iso");
        let ud_path = cd_dir.join("user-data");
        let md_path = cd_dir.join("meta-data");
        let nc_path;

        if let Some(ref netconf) = self.network_config {
            let path = cd_dir.join("network-config");
            std::fs::write(&path, netconf)?;
            nc_path = Some(path);
        } else {
            nc_path = None;
        }

        if let Some(ref userdata) = self.userdata {
            std::fs::write(&ud_path, userdata)?;
        } else {
            // write out empty file, since create_iso expects at least a file
            std::fs::write(&ud_path, Vec::new())?;
        }

        std::fs::write(&md_path, &self.metadata.to_bytes()?)?;

        create_iso(&iso_path, &ud_path, &md_path, &nc_path)?;

        std::fs::remove_file(&md_path)?;
        std::fs::remove_file(&ud_path)?;

        if let Some(ref path) = nc_path {
            std::fs::remove_file(path)?;
        }

        std::fs::remove_dir(&cd_dir)?;

        Ok(iso_path)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    instance_id: String,
    local_hostname: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    network_interfaces: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    public_keys: Vec<String>,
}

impl Metadata {
    pub fn new(instance_name: &str) -> Self {
        Self {
            instance_id: instance_name.to_string(),
            local_hostname: instance_name.to_string(),
            network_interfaces: None,
            public_keys: Vec::new(),
        }
    }

    pub fn add_public_key(&mut self, public_key: &str) {
        self.public_keys.push(public_key.to_string());
    }

    pub fn add_network_block(&mut self, network_block: String) {
        self.network_interfaces = Some(network_block);
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        serde_yaml::to_writer(&mut buf, &self)?;
        Ok(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ec2_md() {
        let md = Metadata::new("test123").to_bytes().unwrap();
        assert!(String::from_utf8(md)
            .unwrap()
            .contains("instance-id: test123"));
    }
}
