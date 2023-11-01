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

use hex;
use rand::{thread_rng, Rng};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Mac {
    octets: [u8; 6],
}

impl Mac {
    /// Generate and return a new random MAC address
    pub fn gen() -> Self {
        let mut rng = thread_rng();

        let mac: [u8; 6] = [
            0x00,
            0x16,
            0x3e,
            rng.gen_range(0x00..0x7f),
            rng.gen_range(0x00..0xff),
            rng.gen_range(0x00..0xff),
        ];

        Self { octets: mac }
    }

    /// Derives and returns an IPv6 Stateless Address Autoconfiguration address
    /// from this Mac address
    pub fn to_ipv6_slaac_addr(&self) -> String {
        let octets = &self.octets;

        // flip 7th bit of mac
        let flipped = octets[0] | 0b0000_00010;
        let addr = [
            (flipped, octets[1]),
            (octets[2], 0xff),
            (0xfe, octets[3]),
            (octets[4], octets[5]),
        ];

        let s = addr
            .into_iter()
            .map(|s| hex::encode([s.0, s.1]))
            .collect::<Vec<_>>()
            .join(":");
        let s = "fe80::".to_owned() + s.trim_start_matches("0");

        s
    }
}

impl std::fmt::Display for Mac {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.octets.map(|o| hex::encode([o])).join(":"))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MacParseError;

impl std::str::FromStr for Mac {
    type Err = MacParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<_> = s
            .split(":")
            .map(|o| hex::decode(o))
            .flatten()
            .flatten()
            .collect();
        let octets: [u8; 6] = v.try_into().map_err(|_| MacParseError)?;

        Ok(Mac { octets })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_mac() {
        let mac = Mac::gen().to_string();
        eprintln!("{}", mac);
        assert!(mac.starts_with("00:16:3e"));
    }

    #[test]
    fn ipv6_from_mac() {
        let ts = [
            ("00:16:3e:23:59:0f", "fe80::216:3eff:fe23:590f"),
            ("00:16:3e:5f:5d:47", "fe80::216:3eff:fe5f:5d47"),
        ];

        for t in ts {
            let res = t.0.parse::<Mac>().unwrap().to_ipv6_slaac_addr();
            assert_eq!(res, t.1);
        }
    }

    #[test]
    fn test_copy() {
        let mac = Mac::gen();
        let mac2 = mac;
        assert_eq!(mac, mac2);
    }

    #[test]
    fn parse_ok() {
        let s = "00:11:22:33:44:55";
        let mac: Mac = s.parse().unwrap();

        assert_eq!(s, mac.to_string());
    }

    #[test]
    #[should_panic(expected = "MacParseError")]
    fn parse_invalid() {
        let s = "00:11:22:33:44:zz";
        let mac: Mac = s.parse().unwrap();
    }

    #[test]
    #[should_panic(expected = "MacParseError")]
    fn parse_too_long() {
        let s = "00:11:22:33:44:55:66";
        let mac: Mac = s.parse().unwrap();
    }
}
