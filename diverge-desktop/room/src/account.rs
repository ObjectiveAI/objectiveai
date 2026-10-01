//! An account: you, across every Mac you use, made from your recovery words.
//!
//! - **The root.** An Ed25519 key derived from the words' seed by SLIP-10's
//!   hardened derivation (the only kind Ed25519 has), at [`ROOT_PATH`]. It
//!   signs the genesis and the device lists. Whoever makes it drops it as
//!   soon as those are signed; it is never kept.
//! - **The genesis.** The root's statement that the account exists. The
//!   account's id is the digest of the genesis, so nobody can make another
//!   account with the same id.
//! - **Device lists.** The root names the keys that act for the account on
//!   each Mac, with a sequence number. The list with the highest sequence
//!   is the one that counts; one no newer than the list already held is
//!   refused.
//!
//! What's here is shared: the app makes an account, and anything that
//! checks one (a room, the app reading someone else's) uses [`Proof::check`].

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha512;
use zeroize::Zeroize;

use crate::seal::{Key, Keypair, Statement, canonical, digest};

/// The statement kind of an account's genesis.
pub const GENESIS: &str = "account";
/// The statement kind of an account's device list.
pub const DEVICES: &str = "devices";

/// Where the account's root sits under the words: the first hardened child.
/// Other accounts made from the same words take the hardened indexes after it.
pub const ROOT_PATH: &[u32] = &[0];

/// The hardened bit: SLIP-10 writes index `i` hardened as `i + 2^31`.
const HARDENED: u32 = 0x8000_0000;

/// One SLIP-10 node: a secret key and its chain code. Wiped when dropped.
pub struct Node {
    pub key: [u8; 32],
    pub chain: [u8; 32],
}

impl Drop for Node {
    fn drop(&mut self) {
        self.key.zeroize();
        self.chain.zeroize();
    }
}

fn hmac512(key: &[u8], parts: &[&[u8]]) -> Node {
    let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(key).expect("HMAC takes a key of any length");
    for part in parts {
        mac.update(part);
    }
    let mut out: [u8; 64] = mac.finalize().into_bytes().into();
    let mut node = Node { key: [0; 32], chain: [0; 32] };
    node.key.copy_from_slice(&out[..32]);
    node.chain.copy_from_slice(&out[32..]);
    out.zeroize();
    node
}

/// SLIP-10 for Ed25519: the node at `path` under `seed`. Every index is
/// hardened (given here without the hardened bit), since Ed25519 has no
/// other kind; an index that already carries the bit is refused.
pub fn derive(seed: &[u8], path: &[u32]) -> Result<Node, String> {
    let mut node = hmac512(b"ed25519 seed", &[seed]);
    for &index in path {
        if index >= HARDENED {
            return Err("an index is given without its hardened bit".into());
        }
        node = hmac512(&node.chain, &[&[0u8], &node.key, &(index | HARDENED).to_be_bytes()]);
    }
    Ok(node)
}

/// The account's root key, from the words' seed.
pub fn root(seed: &[u8]) -> Keypair {
    let node = derive(seed, ROOT_PATH).expect("the root's path is hardened");
    Keypair::from_secret_bytes(&node.key)
}

/// The root's statement that the account exists.
pub fn genesis(root: &Keypair, at: DateTime<Utc>) -> Statement {
    Statement::make(root, GENESIS, json!({ "root": root.key(), "at": at }))
}

/// An account's id: the digest of its genesis, signature and all.
pub fn account_id(genesis: &Statement) -> String {
    digest(canonical(&serde_json::to_value(genesis).unwrap_or_default()).as_bytes())
}

/// The root's list of the keys that act for the account.
pub fn device_list(root: &Keypair, account: &str, sequence: u64, devices: &[Key]) -> Statement {
    Statement::make(root, DEVICES, json!({ "account": account, "sequence": sequence, "devices": devices }))
}

/// What a proof says, once it checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub root: Key,
    pub sequence: u64,
    pub devices: Vec<Key>,
}

/// An account's proof: its genesis and the newest device list its root signed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    pub genesis: Statement,
    pub devices: Statement,
}

fn is_key(v: &Value) -> bool {
    v.as_str().is_some_and(|k| k.len() == 64 && k.bytes().all(|b| b.is_ascii_hexdigit()))
}

impl Proof {
    /// The account's id.
    pub fn id(&self) -> String {
        account_id(&self.genesis)
    }

    /// The account, if the genesis and the list both hold: each signed by
    /// the root the genesis names, the list for this account.
    pub fn check(&self) -> Result<Account, String> {
        let root = self.root()?;
        let (sequence, devices) = self.list(&self.devices, &root)?;
        Ok(Account { id: self.id(), root, sequence, devices })
    }

    fn root(&self) -> Result<Key, String> {
        let g = &self.genesis;
        if g.kind != GENESIS || g.field("root") != Some(g.key.as_str()) || !g.holds() {
            return Err("this account's genesis doesn't hold".into());
        }
        Ok(g.key.clone())
    }

    fn list(&self, list: &Statement, root: &str) -> Result<(u64, Vec<Key>), String> {
        if list.kind != DEVICES || list.key != root || !list.holds() {
            return Err("this device list isn't signed by the account's root".into());
        }
        if list.field("account") != Some(self.id().as_str()) {
            return Err("this device list is for another account".into());
        }
        let sequence = list.body.get("sequence").and_then(Value::as_u64).filter(|s| *s >= 1).ok_or("this device list has no sequence")?;
        let devices = list.body.get("devices").and_then(Value::as_array).filter(|d| !d.is_empty() && d.iter().all(is_key)).ok_or("this device list names no keys")?;
        Ok((sequence, devices.iter().filter_map(Value::as_str).map(str::to_owned).collect()))
    }

    /// Take a newer device list. One no newer than the list held is
    /// refused: the same list again changes nothing, anything else is.
    pub fn accept(&mut self, list: Statement) -> Result<(), String> {
        let root = self.root()?;
        let (held, _) = self.list(&self.devices, &root)?;
        let (sequence, _) = self.list(&list, &root)?;
        if list == self.devices {
            return Ok(());
        }
        if sequence <= held {
            return Err(format!("this device list is number {sequence}, and number {held} is already held"));
        }
        self.devices = list;
        Ok(())
    }

    /// Whether a key acts for the account.
    pub fn names(&self, key: &str) -> bool {
        self.check().is_ok_and(|a| a.devices.iter().any(|d| d == key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(s: &str) -> [u8; 32] {
        hex::decode(s).unwrap().try_into().unwrap()
    }

    /// One chain of a test vector: its path, chain code, private key and public key.
    type Chain = (&'static [u32], &'static str, &'static str, &'static str);

    /// SLIP-10's own test vectors for Ed25519: for each path, the chain
    /// code, the private key and the public key (with its 00 prefix).
    #[test]
    fn slip10_ed25519_test_vectors() {
        let vectors: [(&str, &[Chain]); 2] = [
            (
                "000102030405060708090a0b0c0d0e0f",
                &[
                    (&[], "90046a93de5380a72b5e45010748567d5ea02bbf6522f979e05c0d8d8ca9fffb", "2b4be7f19ee27bbf30c667b642d5f4aa69fd169872f8fc3059c08ebae2eb19e7", "00a4b2856bfec510abab89753fac1ac0e1112364e7d250545963f135f2a33188ed"),
                    (&[0], "8b59aa11380b624e81507a27fedda59fea6d0b779a778918a2fd3590e16e9c69", "68e0fe46dfb67e368c75379acec591dad19df3cde26e63b93a8e704f1dade7a3", "008c8a13df77a28f3445213a0f432fde644acaa215fc72dcdf300d5efaa85d350c"),
                    (&[0, 1], "a320425f77d1b5c2505a6b1b27382b37368ee640e3557c315416801243552f14", "b1d0bad404bf35da785a64ca1ac54b2617211d2777696fbffaf208f746ae84f2", "001932a5270f335bed617d5b935c80aedb1a35bd9fc1e31acafd5372c30f5c1187"),
                    (&[0, 1, 2], "2e69929e00b5ab250f49c3fb1c12f252de4fed2c1db88387094a0f8c4c9ccd6c", "92a5b23c0b8a99e37d07df3fb9966917f5d06e02ddbd909c7e184371463e9fc9", "00ae98736566d30ed0e9d2f4486a64bc95740d89c7db33f52121f8ea8f76ff0fc1"),
                    (&[0, 1, 2, 2], "8f6d87f93d750e0efccda017d662a1b31a266e4a6f5993b15f5c1f07f74dd5cc", "30d1dc7e5fc04c31219ab25a27ae00b50f6fd66622f6e9c913253d6511d1e662", "008abae2d66361c879b900d204ad2cc4984fa2aa344dd7ddc46007329ac76c429c"),
                    (&[0, 1, 2, 2, 1000000000], "68789923a0cac2cd5a29172a475fe9e0fb14cd6adb5ad98a3fa70333e7afa230", "8f94d394a8e8fd6b1bc2f3f49f5c47e385281d5c17e65324b0f62483e37e8793", "003c24da049451555d51a7014a37337aa4e12d41e485abccfa46b47dfb2af54b7a"),
                ],
            ),
            (
                "fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542",
                &[
                    (&[], "ef70a74db9c3a5af931b5fe73ed8e1a53464133654fd55e7a66f8570b8e33c3b", "171cb88b1b3c1db25add599712e36245d75bc65a1a5c9e18d76f9f2b1eab4012", "008fe9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a"),
                    (&[0], "0b78a3226f915c082bf118f83618a618ab6dec793752624cbeb622acb562862d", "1559eb2bbec5790b0c65d8693e4d0875b1747f4970ae8b650486ed7470845635", "0086fab68dcb57aa196c77c5f264f215a112c22a912c10d123b0d03c3c28ef1037"),
                    (&[0, 2147483647], "138f0b2551bcafeca6ff2aa88ba8ed0ed8de070841f0c4ef0165df8181eaad7f", "ea4f5bfe8694d8bb74b7b59404632fd5968b774ed545e810de9c32a4fb4192f4", "005ba3b9ac6e90e83effcd25ac4e58a1365a9e35a3d3ae5eb07b9e4d90bcf7506d"),
                    (&[0, 2147483647, 1], "73bd9fff1cfbde33a1b846c27085f711c0fe2d66fd32e139d3ebc28e5a4a6b90", "3757c7577170179c7868353ada796c839135b3d30554bbb74a4b1e4a5a58505c", "002e66aa57069c86cc18249aecf5cb5a9cebbfd6fadeab056254763874a9352b45"),
                    (&[0, 2147483647, 1, 2147483646], "0902fe8a29f9140480a00ef244bd183e8a13288e4412d8389d140aac1794825a", "5837736c89570de861ebc173b1086da4f505d4adb387c6a1b1342d5e4ac9ec72", "00e33c0f7d81d843c572275f287498e8d408654fdf0d1e065b84e2e6f157aab09b"),
                    (&[0, 2147483647, 1, 2147483646, 2], "5d70af781f3a37b829f0d060924d5e960bdc02e85423494afc0b1a41bbe196d4", "551d333177df541ad876a60ea71f00447931c0a9da16f227c11ea080d7391b8d", "0047150c75db263559a70d5778bf36abbab30fb061ad69f69ece61a72b0cfa4fc0"),
                ],
            ),
        ];
        for (seed, chains) in vectors {
            let seed = hex::decode(seed).unwrap();
            for (path, chain, private, public) in chains {
                let node = derive(&seed, path).unwrap();
                assert_eq!(node.chain, hex32(chain), "chain code at {path:?}");
                assert_eq!(node.key, hex32(private), "private key at {path:?}");
                assert_eq!(format!("00{}", Keypair::from_secret_bytes(&node.key).key()), *public, "public key at {path:?}");
            }
        }
        assert!(derive(&[0; 16], &[HARDENED]).is_err(), "an index is given without its hardened bit");
    }

    fn made() -> (Keypair, Proof) {
        let root = root(&[7; 64]);
        let genesis = genesis(&root, Utc::now());
        let id = account_id(&genesis);
        let devices = device_list(&root, &id, 1, &[Keypair::from_seed("this mac").key()]);
        (root, Proof { genesis, devices })
    }

    #[test]
    fn the_account_id_is_the_digest_of_its_genesis() {
        let (_, proof) = made();
        let account = proof.check().unwrap();
        assert_eq!(account.id, digest(canonical(&serde_json::to_value(&proof.genesis).unwrap()).as_bytes()));
        assert_eq!((account.sequence, account.devices.clone()), (1, vec![Keypair::from_seed("this mac").key()]));
        assert!(proof.names(&Keypair::from_seed("this mac").key()));
        // Change anything in the genesis and it's another account, or none.
        let mut other = proof.clone();
        other.genesis.body["at"] = json!(Utc::now() + chrono::TimeDelta::days(1));
        assert_ne!(other.id(), account.id);
        assert!(other.check().is_err(), "its signature no longer holds");
    }

    #[test]
    fn a_device_list_lower_than_the_one_held_is_refused() {
        let (root, mut proof) = made();
        let id = proof.id();
        let second = device_list(&root, &id, 2, &[Keypair::from_seed("this mac").key(), Keypair::from_seed("a second mac").key()]);
        proof.accept(second.clone()).unwrap();
        assert_eq!(proof.check().unwrap().sequence, 2);
        let first = device_list(&root, &id, 1, &[Keypair::from_seed("this mac").key()]);
        assert!(proof.accept(first).is_err(), "a lower sequence");
        let rival = device_list(&root, &id, 2, &[Keypair::from_seed("this mac").key()]);
        assert!(proof.accept(rival).is_err(), "the same sequence, saying something else");
        proof.accept(second).unwrap();
        assert!(proof.names(&Keypair::from_seed("a second mac").key()), "the same list again changes nothing");
        // Only the root signs a list, and only for its own account.
        let stranger = Keypair::from_seed("a stranger");
        assert!(proof.accept(device_list(&stranger, &id, 3, &[stranger.key()])).is_err());
        assert!(proof.accept(device_list(&root, "another account", 3, &[stranger.key()])).is_err());
        assert!(proof.accept(device_list(&root, &id, 3, &[])).is_err(), "a list names someone");
        assert!(!proof.names(&stranger.key()));
    }
}
