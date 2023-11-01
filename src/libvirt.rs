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

use std::io::Cursor;
use std::path::Path;

use quick_xml::writer::Writer;
use virt::{connect::Connect, domain::Domain};

use crate::error::Error;

pub struct DomainBuilder {
    pub name: String,
    pub cpus: u32,
    pub memory_bytes: u64,
    pub image_file: String,

    network_xml: String,
    block_device_xml: String,

    metadata_api: bool,
}

impl DomainBuilder {
    pub fn new<P: AsRef<Path>>(name: &str, cpus: u32, memory_bytes: u64, image_file: P) -> Self {
        Self {
            name: name.to_string(),
            cpus,
            memory_bytes,
            image_file: image_file.as_ref().to_str().unwrap().to_string(),
            network_xml: String::new(),
            block_device_xml: String::new(),
            metadata_api: false,
        }
    }

    pub fn add_cdrom_from_iso<P: AsRef<Path>>(&mut self, iso_file_path: P) -> Result<(), Error> {
        let iso_path_str = iso_file_path.as_ref().to_str().unwrap();

        let mut w = Writer::new(Cursor::new(Vec::new()));
        w.create_element("disk")
            .with_attribute(("type", "file"))
            .with_attribute(("device", "cdrom"))
            .write_inner_content(|w| {
                w.create_element("source")
                    .with_attribute(("file", iso_path_str))
                    .write_empty()?;

                w.create_element("readonly").write_empty()?;

                w.create_element("target")
                    .with_attribute(("dev", "hdc"))
                    .with_attribute(("bus", "ide"))
                    .write_empty()?;

                Ok(())
            })?;

        let xml = String::from_utf8(w.into_inner().into_inner())?;
        self.block_device_xml.push_str(&xml);

        Ok(())
    }

    pub fn render(&self) -> String {
        let smbios;

        if self.metadata_api {
            smbios = r#"
  <sysinfo type="smbios">
    <bios>
      <entry name="vendor">BigIron</entry>
    </bios>
    <system>
      <entry name="product">OpenStack Nova</entry>
      <entry name="manufacturer">BigIron</entry>
    </system>
  </sysinfo>"#;
        } else {
            smbios = "<sysinfo type=\"smbios\"></sysinfo>";
        }

        format!(
            r#"
<domain type="kvm">
  <name>{name}</name>
  <memory unit="bytes">{memory_bytes}</memory>
  <currentMemory unit="bytes">{memory_bytes}</currentMemory>
  <vcpu>{cpus}</vcpu>
  <os>
    <smbios mode="sysinfo"/>
    <type arch="x86_64" machine="pc">hvm</type>
    <boot dev="hd"/>
  </os>
  <features>
    <acpi/>
    <apic/>
  </features>
  <clock offset="utc"/>
  <pm>
    <suspend-to-mem enabled="no"/>
    <suspend-to-disk enabled="no"/>
  </pm>
  <devices>
    <disk type="file" device="disk">
      <driver name="qemu" type="qcow2" cache="writeback"/>
      <source file="{image_file}"/>
      <target dev="vda" bus="virtio"/>
    </disk>
    {block_devices}
    <serial type="pty">
      <source path="/dev/pts/0"/>
      <target type="isa-serial" port="0"/>
    </serial>
    <input type="keyboard" bus="ps2"/>
    <input type="mouse" bus="ps2"/>
    {network_xml}
    <memballoon model="virtio"/>
  </devices>
  {smbios_block}
</domain>
        "#,
            name = &self.name,
            memory_bytes = self.memory_bytes,
            cpus = self.cpus,
            image_file = &self.image_file,
            network_xml = self.network_xml,
            smbios_block = smbios,
            block_devices = self.block_device_xml,
        )
    }

    pub fn build(self) -> Result<(), Error> {
        let domxml = self.render();

        let c = Connect::open("")?;
        let _dom = Domain::create_xml(&c, &domxml.to_string(), 0)?;
        Ok(())
    }

    pub fn add_bridged_interface(&mut self, name: &str, macaddr: &str) {
        let xml = format!(
            r#"<interface type="bridge">
      <source bridge="{name}"/>
      <mac address="{macaddr}"/>
      <model type="virtio"/>
    </interface>"#,
            name = name,
            macaddr = macaddr
        );

        self.network_xml.push_str(&xml);
    }

    pub fn add_macvtap_interface(&mut self, name: &str, macaddr: &str) {
        let xml = format!(
            r#"<interface type="direct">
      <source dev="{name}" mode="bridge"/>
      <mac address="{macaddr}"/>
      <model type="virtio"/>
    </interface>"#,
            name = name,
            macaddr = macaddr
        );

        self.network_xml.push_str(&xml);
    }

    pub fn add_file_backed_storage<P: AsRef<Path>>(&mut self, path: P, target_dev: &str) {
        self.add_storage(path, target_dev, "file", "file")
            .expect("error building storage XML definition");
    }

    pub fn add_block_backed_storage<P: AsRef<Path>>(&mut self, path: P, target_dev: &str) {
        self.add_storage(path, target_dev, "block", "dev")
            .expect("error building storage XML definition");
    }

    fn add_storage<P: AsRef<Path>>(
        &mut self,
        path: P,
        target_dev: &str,
        disk_type: &str,
        source_type: &str,
    ) -> Result<(), Error> {
        let path_str = path.as_ref().to_str().unwrap();

        let mut w = Writer::new(Cursor::new(Vec::new()));
        w.create_element("disk")
            .with_attribute(("type", disk_type))
            .with_attribute(("device", "disk"))
            .write_inner_content(|w| {
                w.create_element("source")
                    .with_attribute((source_type, path_str))
                    .write_empty()?;

                w.create_element("target")
                    .with_attribute(("dev", target_dev))
                    .with_attribute(("bus", "virtio"))
                    .write_empty()?;

                Ok(())
            })?;

        let xml = String::from_utf8(w.into_inner().into_inner())?;
        self.block_device_xml.push_str(&xml);

        Ok(())
    }
}

pub fn destroy(name: &str) -> Result<(), Error> {
    let c = Connect::open("")?;
    let dom = Domain::lookup_by_name(&c, name);
    if let Err(ref e) = dom {
        if e.to_string().contains("Domain not found") {
            return Ok(());
        }
        dom?;
    } else {
        dom.unwrap().destroy()?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_build_bridged() {
        let mut d = DomainBuilder::new("test123", 4, 8 * 1024 * 1024 * 1024, "test123.qcow2");
        d.add_bridged_interface("obsbr0", "00:11:22:33:44:55");
        let xml = d.render();

        eprintln!("{}", &xml);

        assert!(xml.contains("source bridge=\"obsbr0\""));
    }

    #[test]
    pub fn test_build_macvtap() {
        let mut d = DomainBuilder::new("test123", 4, 8 * 1024 * 1024 * 1024, "test123.qcow2");
        d.add_macvtap_interface("eth0", "00:11:22:33:44:55");
        let xml = d.render();

        eprintln!("{}", &xml);

        assert!(xml.contains("source dev=\"eth0\" mode=\"bridge\""));
    }
}
