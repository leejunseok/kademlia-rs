use std::fmt::{Debug,Error,Formatter};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use rand;
use rustc_serialize::{Decodable,Decoder,Encodable,Encoder};

use ::K;

#[derive(Hash,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Key([u8; K]);

impl Key {
    /// Returns a random, K long byte string.
    pub fn random() -> Key {
        let mut res = [0; K];
        for i in 0usize..K {
            res[i] = rand::random::<u8>();
        }
        Key(res)
    }

    /// XORs two Keys
    pub fn dist(&self, y: Key) -> Distance{
        let mut res = [0; K];
        for i in 0usize..K {
            res[i] = self.0[i] ^ y.0[i];
        }
        Distance(res)
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter().rev() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Key {
        let mut hasher = Sha1::new();
        hasher.input_str(&s);
        let mut hash = [0u8; K];
        for (i, b) in hasher.result_str().as_bytes().iter().take(K).enumerate() {
            hash[i] = *b;
        }
        Key(hash)
    }
}

impl Decodable for Key {
    fn decode<D: Decoder>(d: &mut D) -> Result<Key, D::Error> {
        d.read_seq(|d, len| {
            if len != K {
                return Err(d.error("Wrong length key!"));
            }
            let mut ret = [0; K];
            for i in 0..K {
                ret[i] = try!(d.read_seq_elt(i, Decodable::decode));
            }
            Ok(Key(ret))
        })
    }
}

impl Encodable for Key {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(K, |s| {
            for i in 0..K {
                try!(s.emit_seq_elt(i, |s| self.0[i].encode(s)));
            }
            Ok(())
        })
    }
}

#[derive(Hash,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Distance([u8; K]);

impl Distance {
    pub fn zeroes_in_prefix(&self) -> usize {
        for i in 0..K {
            for j in 8usize..0 {
                if (self.0[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }
        K * 8 - 1
    }
}

impl Debug for Distance {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}
