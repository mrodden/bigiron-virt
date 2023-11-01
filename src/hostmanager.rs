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

use tracing::info;
use url::Url;

use crate::api::models::Machine;
use crate::configdrive;
use crate::error::Error;
use crate::image::repo::Directory;
use crate::libvirt;
use crate::mac::Mac;
use crate::network_config;
use crate::vmstore::VMStore;

pub struct HostManager {
    vmstore: VMStore,
    imagestore: Directory,
}

pub type MachineList = Vec<MachineStatus>;

pub struct MachineStatus {
    pub id: String,
    pub status: String,
}

impl HostManager {
    pub fn new() -> Result<Self, Error> {
        let vsp = "/var/lib/bigiron-virt/instances";
        let isp = "/var/lib/bigiron-virt/images";

        Ok(Self {
            vmstore: VMStore::new(&vsp)?,
            imagestore: Directory::new(&isp)?,
        })
    }

    pub fn create_machine(&mut self, machine: &mut Machine) -> Result<(), Error> {
        let name = &machine.metadata.name;

        // ensure base image imported to repo
        let image_url = Url::parse(&machine.spec.image.url)?;
        let image_base_id = self
            .imagestore
            .add_image(&image_url, &machine.spec.image.hash)?;

        // create instance storage directory
        let instance_dir = self.vmstore.new_instance(name)?;

        // create instance image from base
        let image_size = match machine.spec.image.resize {
            None => None,
            Some(ref size_string) => Some(crate::api::models::to_size(size_string)?),
        };

        let image_path = self.vmstore.create_instance_image(
            name,
            self.imagestore.get_image(&image_base_id)?,
            image_size,
        )?;

        // create base vm spec
        let mut d = libvirt::DomainBuilder::new(
            name,
            machine.spec.cpu,
            crate::api::models::to_size(&machine.spec.memory)?,
            image_path,
        );

        let mut bridged_nic_info = None;

        // network config
        if let Some(nics) = &mut machine.spec.nics {
            for nic in nics.iter_mut() {
                nic.macaddress = Mac::gen().to_string();

                match nic.kind.as_str() {
                    "Bridge" => {
                        d.add_bridged_interface(&nic.parent, &nic.macaddress);
                        bridged_nic_info = Some(nic.macaddress.clone());
                    }
                    "Macvtap" => {
                        d.add_macvtap_interface(&nic.parent, &nic.macaddress);
                    }
                    &_ => {}
                }
            }
        }

        let netconf = network_config::build_net_config(&machine.spec.nics)?;

        // create config drive
        let mut builder = configdrive::Builder::new(name);

        if !netconf.is_empty() {
            builder.add_network_config(netconf);
        }

        if let Some(ref userdata) = machine.spec.userdata {
            builder.add_userdata(userdata.as_bytes().to_vec());
        }

        let cd_path = builder.build(instance_dir)?.canonicalize()?;

        // attach config drive
        d.add_cdrom_from_iso(&cd_path)?;

        // attach storage devices
        if let Some(storages) = &machine.spec.storage {
            let drive_letter_start: u8 = 98; // "b" in ASCII
            use crate::api::models::StorageKind;
            for (i, store) in storages.iter().enumerate() {
                if i > 24 {
                    panic!("not enough drive letters for storage drives");
                }
                // i already fits from above check
                let i_u8: u8 = i.try_into().unwrap();

                let v = [118, 100, drive_letter_start + i_u8];
                let target_name = std::str::from_utf8(&v).unwrap();

                match store {
                    StorageKind::File(ref file) => {
                        d.add_file_backed_storage(&file.path, &target_name);
                    }
                    StorageKind::Block(ref block) => {
                        d.add_block_backed_storage(&block.path, &target_name);
                    }
                }
            }
        }

        // define/create domain
        d.build()?;

        if let Some(info) = bridged_nic_info {
            match info.parse::<Mac>() {
                Ok(mac) => info!("IPv6 SLAAC: {}", mac.to_ipv6_slaac_addr()),
                Err(_) => {}
            }
        }

        Ok(())
    }

    pub fn destroy_machine(&mut self, id: &str) -> Result<(), Error> {
        // destroy in libvirt
        libvirt::destroy(id)?;

        // destroy in VM store
        self.vmstore.remove_instance(id)?;

        Ok(())
    }

    pub fn list_machines(&self) -> Result<MachineList, Error> {
        let ids = self.vmstore.list_instances()?;

        let get_status = |entry: String| MachineStatus {
            id: entry,
            status: String::from("unknown"),
        };

        let list = ids.into_iter().map(get_status).collect();

        Ok(list)
    }
}
