//! Deterministic ObjectId to uuidv7 mapping for the mongo import.
//!
//! The import has to translate every id, and every reference to one, without keeping a lookup
//! table of the whole database in memory. Deriving the uuid from the ObjectId's own bytes
//! makes the mapping a pure function, so a re-run produces the same rows, a foreign key can be
//! translated without having seen the row it points at, and a mismatch is debuggable by hand.
//!
//! ObjectId is 4 bytes of second-resolution timestamp, 5 random bytes and a 3-byte counter.
//! uuidv7 is 48 bits of millisecond timestamp, 12 bits of rand_a and 62 bits of rand_b. The
//! timestamp carries over (which keeps the creation ordering that made uuidv7 worth choosing)
//! and the remaining 8 bytes fit in the 74 available bits with room to spare, so the mapping
//! is injective: distinct ObjectIds cannot collide.

use mongodb::bson::oid::ObjectId;
use uuid::Uuid;

pub fn uuid_from_object_id(oid: &ObjectId) -> Uuid {
    let bytes = oid.bytes();

    let seconds = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64;
    let millis = seconds * 1000;

    // the 5 random bytes and the 3 counter bytes, which are what make an ObjectId unique
    // within a second
    let tail = u64::from_be_bytes(bytes[4..12].try_into().expect("ObjectId is 12 bytes"));
    let rand_a = ((tail >> 52) & 0x0fff) as u16;
    let rand_b = tail & 0x000f_ffff_ffff_ffff;

    let mut out = [0u8; 16];
    out[0..6].copy_from_slice(&millis.to_be_bytes()[2..8]);
    out[6] = 0x70 | (rand_a >> 8) as u8;
    out[7] = (rand_a & 0xff) as u8;
    out[8] = 0x80;
    out[9..16].copy_from_slice(&rand_b.to_be_bytes()[1..8]);

    Uuid::from_bytes(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Inverse of [`uuid_from_object_id`]. Only used to prove the mapping loses nothing; the
    /// import never needs to go back.
    fn object_id_from_uuid(uuid: &Uuid) -> ObjectId {
        let b = uuid.as_bytes();

        let mut millis = [0u8; 8];
        millis[2..8].copy_from_slice(&b[0..6]);
        let seconds = (u64::from_be_bytes(millis) / 1000) as u32;

        let rand_a = (((b[6] & 0x0f) as u16) << 8) | b[7] as u16;
        let mut rb = [0u8; 8];
        rb[1..8].copy_from_slice(&b[9..16]);
        let rand_b = u64::from_be_bytes(rb);

        let tail = ((rand_a as u64) << 52) | rand_b;

        let mut out = [0u8; 12];
        out[0..4].copy_from_slice(&seconds.to_be_bytes());
        out[4..12].copy_from_slice(&tail.to_be_bytes());
        ObjectId::from_bytes(out)
    }

    #[test]
    fn is_a_wellformed_uuidv7() {
        let u = uuid_from_object_id(&ObjectId::new());
        assert_eq!(u.get_version_num(), 7, "{u}");
        // RFC 9562 variant bits are 0b10
        assert_eq!(u.as_bytes()[8] & 0xc0, 0x80, "{u}");
    }

    #[test]
    fn loses_nothing_from_the_object_id() {
        for _ in 0..1000 {
            let oid = ObjectId::new();
            assert_eq!(object_id_from_uuid(&uuid_from_object_id(&oid)), oid);
        }
    }

    /// The whole point of deriving rather than generating: a second run of the import has to
    /// produce the same rows, or re-running it would duplicate the database.
    /// The whole point of deriving rather than generating: a second run of the import has to
    /// produce the same rows, or re-running it would duplicate the database. Pinned to a
    /// literal so a change to the packing shows up here rather than as a silent reimport.
    #[test]
    fn is_stable_across_runs() {
        let oid = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        assert_eq!(
            uuid_from_object_id(&oid).to_string(),
            "013a7092-e8d8-7bcf-8008-6cd799439011"
        );
    }

    #[test]
    fn distinct_object_ids_stay_distinct() {
        let ids: Vec<_> = (0..10_000).map(|_| ObjectId::new()).collect();
        let uuids: HashSet<_> = ids.iter().map(uuid_from_object_id).collect();
        assert_eq!(uuids.len(), ids.len(), "the mapping collided");
    }

    /// Two ObjectIds generated a second apart have to keep their order, since preserving
    /// creation ordering is why uuidv7 was picked over v4.
    #[test]
    fn preserves_creation_order() {
        let early = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        let late = ObjectId::parse_str("607f1f77bcf86cd799439011").unwrap();
        assert!(uuid_from_object_id(&early) < uuid_from_object_id(&late));
    }
}
