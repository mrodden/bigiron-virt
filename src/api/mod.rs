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

use serde_yaml;

pub mod models;
use models::{Machine, Resource};

use crate::error::Error;
use crate::hostmanager::{HostManager, MachineStatus};

pub fn resources_from_yaml(yaml: &str) -> Result<Vec<Resource>, Error> {
    let mut rs = Vec::new();

    for res in yaml.split("---\n") {
        if res.is_empty() {
            continue;
        }

        let r = serde_yaml::from_str(&res)?;
        rs.push(r);
    }

    Ok(rs)
}

pub fn create_from_yaml(yaml: &str) -> Result<(), Error> {
    let resources = resources_from_yaml(yaml).unwrap();

    let mut hm = HostManager::new()?;

    for res in resources {
        match res {
            Resource::Machine(mut m) => {
                hm.create_machine(&mut m)?;
            }
        }
    }

    Ok(())
}

pub fn list_machines() -> Result<Vec<MachineStatus>, Error> {
    let hm = HostManager::new()?;
    Ok(hm.list_machines()?)
}

pub fn destroy_machine(id: &str) -> Result<(), Error> {
    let mut hm = HostManager::new()?;
    Ok(hm.destroy_machine(id)?)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    pub fn test_resources_from_yaml() {
        let inp = "---
        kind: Machine
        metadata:
          name: vm1
        spec:
          cpu: 4
          memory: 512Mi
          image:
            url: file:///vm1.qcow2
            hash: abc1234
        ---
        kind: Machine
        metadata:
          name: vm2
        spec:
          cpu: 4
          memory: 512Mi
          image:
            url: file:///vm2.qcow2
            hash: abc1234
        ";

        let rs = resources_from_yaml(&inp).unwrap();

        assert!(rs.len() == 2);

        for r in rs {
            match r {
                models::Resource::Machine(m) => {
                    if m.metadata.name == "vm1" {
                        assert!(m.spec.image.url.contains("vm1"));
                    }
                    if m.metadata.name == "vm2" {
                        assert!(m.spec.image.url.contains("vm2"));
                    }
                }
            }
        }
    }
}
