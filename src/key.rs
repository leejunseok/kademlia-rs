use std::fmt::{Debug,Error,Formatter};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use rand;
use rustc_serialize::{Decodable,Decoder,Encodable,Encoder};
use rustc_serialize::hex::FromHex;

use ::KEY_LEN;

#[derive(Hash,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Key([u8; KEY_LEN]);

impl Key {
    /// Returns a random, KEY_LEN long byte string.
    pub fn random() -> Key {
        let mut res = [0; KEY_LEN];
        for i in 0usize..KEY_LEN {
            res[i] = rand::random::<u8>();
        }
        Key(res)
    }

    /// Returns the hashed Key of data.
    pub fn hash(data: String) -> Key {
        let mut hasher = Sha1::new();
        hasher.input_str(&data);
        let mut hash = [0u8; KEY_LEN];
        for (i, b) in hasher.result_str().as_bytes().iter().take(KEY_LEN).enumerate() {
            hash[i] = *b;
        }
        Key(hash)
    }

    /// XORs two Keys
    pub fn dist(&self, y: Key) -> Distance{
        let mut res = [0; KEY_LEN];
        for i in 0usize..KEY_LEN {
            res[i] = self.0[i] ^ y.0[i];
        }
        Distance(res)
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            write!(f, "{0:02x}", x)?;
        }
        Ok(())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Key {
        let mut ret = [0; KEY_LEN];
        for (i, byte) in s.from_hex().unwrap().iter().enumerate() {
            ret[i] = *byte;
        }
        Key(ret)
    }
}

impl Decodable for Key {
    fn decode<D: Decoder>(d: &mut D) -> Result<Key, D::Error> {
        d.read_seq(|d, len| {
            if len != KEY_LEN {
                return Err(d.error("Wrong length key!"));
            }
            let mut ret = [0; KEY_LEN];
            for i in 0..KEY_LEN {
                ret[i] = d.read_seq_elt(i, Decodable::decode)?;
            }
            Ok(Key(ret))
        })
    }
}

impl Encodable for Key {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(KEY_LEN, |s| {
            for i in 0..KEY_LEN {
                s.emit_seq_elt(i, |s| self.0[i].encode(s))?;
            }
            Ok(())
        })
    }
}

#[derive(Hash,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Distance([u8; KEY_LEN]);

impl Distance {
    pub fn zeroes_in_prefix(&self) -> usize {
        for i in 0..KEY_LEN {
            for j in 8usize..0 {
                if (self.0[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }
        KEY_LEN * 8 - 1
    }
}

impl Debug for Distance {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            write!(f, "{0:02x}", x)?;
        }
        Ok(())
    }
}

impl Decodable for Distance {
    fn decode<D: Decoder>(d: &mut D) -> Result<Distance, D::Error> {
        d.read_seq(|d, len| {
            if len != KEY_LEN {
                return Err(d.error("Wrong length key!"));
            }
            let mut ret = [0; KEY_LEN];
            for i in 0..KEY_LEN {
                ret[i] = d.read_seq_elt(i, Decodable::decode)?;
            }
            Ok(Distance(ret))
        })
    }
}

impl Encodable for Distance {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(KEY_LEN, |s| {
            for i in 0..KEY_LEN {
                s.emit_seq_elt(i, |s| self.0[i].encode(s))?;
            }
            Ok(())
        })
    }
}
