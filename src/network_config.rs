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

use std::collections::HashMap as Map;

use serde::{Deserialize, Serialize};
use serde_yaml;

use crate::api;
use crate::error::Error;

pub fn build_net_config(nics: &Option<Vec<api::models::Nic>>) -> Result<Vec<u8>, Error> {
    let mut buf = Vec::new();

    let nics = match nics {
        Some(n) => n,
        None => {
            return Ok(buf);
        }
    };

    let mut ethers: Map<String, Ethernet> = Map::new();

    for (i, nic) in nics.iter().enumerate() {
        let key = format!("id{}", i);
        let ether = Ethernet::try_from(nic)?;
        let _ = ethers.insert(key, ether);
    }

    let conf = NetworkConfig {
        network: NetworkConfigV2 {
            version: 2,
            ethernets: ethers,
        },
    };

    serde_yaml::to_writer(&mut buf, &conf)?;
    Ok(buf)
}

impl TryFrom<&api::models::Nic> for Ethernet {
    type Error = Error;

    fn try_from(nic: &api::models::Nic) -> Result<Self, self::Error> {
        use api::models::AddressKind;

        let mut s = Ethernet::new_with_mac(&nic.macaddress);

        match nic.address {
            AddressKind::IPv6SLAAC => {
                s.dhcp6 = Some(true);
            }
            AddressKind::IPv4Static(ref v4static) => {
                s.addresses = Some(vec![v4static.addr.clone()]);
                s.gateway4 = Some(v4static.gateway.clone());

                if !v4static.nameservers.is_empty() {
                    s.nameservers = Some(Nameservers {
                        search: None,
                        addresses: v4static.nameservers.clone(),
                    })
                }
            }
        }

        Ok(s)
    }
}

impl Ethernet {
    fn new_with_mac(mac: &str) -> Self {
        let m = MatchBlock {
            macaddress: Some(mac.to_string()),
            name: None,
            driver: None,
        };

        Self {
            r#match: m,
            dhcp4: None,
            dhcp6: None,
            addresses: None,
            gateway4: None,
            gateway6: None,
            nameservers: None,
            routes: None,
            wakeonlan: None,
            set_name: None,
        }
    }
}

// supports a subset of Network Config V2 from here:
// https://cloudinit.readthedocs.io/en/latest/reference/network-config-format-v2.html
#[derive(Deserialize, Serialize, Debug, Clone)]
struct NetworkConfig {
    network: NetworkConfigV2,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct NetworkConfigV2 {
    version: u8,
    ethernets: Map<String, Ethernet>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Ethernet {
    r#match: MatchBlock,

    #[serde(skip_serializing_if = "Option::is_none")]
    dhcp4: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dhcp6: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    addresses: Option<Vec<Address>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gateway4: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gateway6: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    nameservers: Option<Nameservers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    routes: Option<Vec<Route>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    wakeonlan: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "set-name")]
    set_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct MatchBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    macaddress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    driver: Option<String>,
}

type Address = String;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Nameservers {
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<Vec<String>>,
    addresses: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Route {
    to: String,
    via: String,
    metric: u32,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize() {
        let sample = "
network:
  version: 2
  ethernets:
    # opaque ID for physical interfaces, only referred to by other stanzas
    id0:
      match:
        macaddress: '00:11:22:33:44:55'
      wakeonlan: true
      dhcp4: true
      addresses:
        - 192.168.14.2/24
        - 2001:1::1/64
      gateway4: 192.168.14.1
      gateway6: 2001:1::2
      nameservers:
        search: [foo.local, bar.local]
        addresses: [8.8.8.8]
      # static routes
      routes:
        - to: 192.0.2.0/24
          via: 11.0.0.1
          metric: 3
    lom:
      match:
        driver: ixgbe
      # you are responsible for setting tight enough match rules
      # that only match one device if you use set-name
      set-name: lom1
      dhcp6: true";

        let conf: NetworkConfig = serde_yaml::from_str(&sample).unwrap();

        eprintln!("{:#?}", conf);

        assert!(conf.network.ethernets.len() == 2);
        assert!(
            conf.network
                .ethernets
                .get("id0")
                .unwrap()
                .gateway4
                .clone()
                .unwrap()
                == "192.168.14.1"
        );
    }
}
