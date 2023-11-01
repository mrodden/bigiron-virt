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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_yaml;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Resource {
    Machine(Machine),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Machine {
    pub metadata: Metadata,
    pub status: Option<String>,
    pub spec: Spec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub name: String,
}

impl Machine {
    pub fn to_yaml(&self) -> Result<String, Error> {
        let buf = serde_yaml::to_string(self)?;
        return Ok(buf);
    }
}

pub type SizeString = String;

pub fn to_size(s: &str) -> Result<u64, Error> {
    let mut last = &s[s.len() - 1..];
    let nlast = &s[s.len() - 2..s.len() - 1];
    let mut co: u64 = 1000;
    let mut num = &s[..s.len() - 1];

    if last == "i" {
        // binary byte mode
        co = 1024;
        last = nlast;
        num = &s[..s.len() - 2];
    }

    let exp = match last {
        "T" | "t" => 3,
        "G" | "g" => 3,
        "M" | "m" => 2,
        "K" | "k" => 1,
        _ => 0,
    };

    let scalar = num.parse::<u64>()?;
    Ok(scalar * co.pow(exp))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spec {
    pub cpu: u32,
    pub memory: SizeString,
    pub image: Image,
    pub storage: Option<Vec<StorageKind>>,
    pub nics: Option<Vec<Nic>>,
    pub userdata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    pub url: String,
    pub hash: String,
    pub resize: Option<SizeString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum StorageKind {
    File(File),
    Block(Block),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Nic {
    pub kind: String,
    pub parent: String,
    pub address: AddressKind,

    // for internal use only, currently
    #[serde(skip)]
    pub macaddress: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum AddressKind {
    IPv6SLAAC,
    IPv4Static(IPv4Static),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IPv4Static {
    pub addr: String,
    pub gateway: String,

    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub nameservers: Vec<String>,
}

#[cfg(test)]
mod test {

    use super::*;

    const sample: &str = r#"kind: Machine
metadata:
  name: othervm
spec:
  cpu: 4
  memory: 512Mi
  image:
    url: "file:///home/mrodden/projects/bigiron-virt/ubuntu-22.04-server-cloudimg-amd64-disk-kvm.img"
    hash: 754129c5052756ee47a0c395e518bd3413f444dff69b98f8a8fa42f2fa3acc2d
    resize: 100G
  storage:
    - kind: File
      path: "/home/mrodden/projects/bigiron-virt/localfile01.qcow2"
  nics:
    - kind: Bridge
      parent: obsbr0
      address:
        kind: IPv6SLAAC
    - kind: Macvtap
      parent: eth0
      address:
        kind: IPv4Static
        addr: "192.168.3.160/24"
        gateway: "192.168.3.1"
  userdata: |
    #cloud-config
    allow_public_ssh_keys: true
    ssh_pwauth: true
    password: password1
    ssh_authorized_keys:
      - ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDs5piczmyxh4yaDTGqJJsKQjGAq8Sjn/Gl78CvcsPBn2dPZuwzmqYZ1gvSf4z8DNviecDVChWVuJKnzI499Rz3UohEly5fVUntu22XI2GGuksbtClCOHzIzkPdI/1YdI+q9qg62u/gicSKz1a/FS5jdyQIpy5wukotiinuQdnQsAENIZJUMmmpofWW9n5bl4cR4/Ig36GMRrBTQm58HfSmvzaUaR3nnsHAaoksUYVQq1zrkT2xSwXL4xryW1GJS5DlnUfZAASQhROfvr0rOfjm97SMJzQHp9fvUinOi19DT1Uk7M14aT59Kr0igJUKHmdxLhDb8BPqUll6BoQQHq1hhwz60viVb5vTcyCS8BipHkAepxAW+9Ln+M7frjRTxHrXj06EDWB9LSpdPFV6rUqbZGVSKlP+526fQqrxgm4KS7wx7cq8I/yKFWjH/x7sGWnw0a9BxmTgrwJ5pc8zHth3sOBPwE64vnFN5vpfWNr5YMQ4Agv0lGlPBkgJlXMJ/FE= mrodden@bawoo
"#;

    #[test]
    fn serialize() {
        let m = Machine{
            status: None,
            metadata: Metadata{name: "othervm".to_string()},
            spec: Spec{
                cpu: 4,
                memory: "512Mi".to_string(),
                image: Image{
                    url: "file:///home/mrodden/projects/bigiron-virt/ubuntu-22.04-server-cloudimg-amd64-disk-kvm.img".to_string(),
                    hash: "754129c5052756ee47a0c395e518bd3413f444dff69b98f8a8fa42f2fa3acc2d".to_string(),
                    resize: Some("100G".to_string()),
                },
                storage: Some(vec![StorageKind::File(File{
                    path: "/home/mrodden/projects/bigiron-virt/localfile01.qcow2".into(),
                })]),
                nics: None,
                userdata: Some("#cloud-config\nallow_public_ssh_keys: true\n".to_string()),
            },
        };

        let out = serde_yaml::to_string(&m).unwrap();
        eprintln!("{}", out);

        assert!(out.contains("resize: 100G"));
        assert!(out.contains("path: /home/mrodden/projects/bigiron-virt/localfile01.qcow2"));
    }

    #[test]
    fn deserialize() {
        let r: Resource = serde_yaml::from_str(sample).unwrap();
        eprintln!("{:#?}", r);
        let m = match r {
            Resource::Machine(m) => m,
        };

        assert!(m.metadata.name == "othervm");
        assert!(m.spec.cpu == 4);
    }

    #[test]
    fn cycle() {
        let m: Resource = serde_yaml::from_str(sample).unwrap();
        let yam = serde_yaml::to_string(&m).unwrap();
        let m2: Resource = serde_yaml::from_str(sample).unwrap();

        assert!(m2 == m);
    }

    #[test]
    fn test_sizestring_to_size() {
        assert_eq!(to_size("100M").unwrap(), 100_000_000);
        assert_eq!(to_size("10m").unwrap(), 10_000_000);
        assert_eq!(to_size("20G").unwrap(), 20_000_000_000);
        assert_eq!(to_size("12g").unwrap(), 12_000_000_000);
        assert_eq!(to_size("12Gi").unwrap(), 12 * 1024 * 1024 * 1024);

        assert!(to_size("12Timmies").is_err());
    }
}
