#![no_main]

use libfuzzer_sys::fuzz_target;
use nyon_workshop_core::{decode_catalog_pack, encode_catalog_pack};

fuzz_target!(|input: &[u8]| {
    if let Ok(pack) = decode_catalog_pack(input) {
        let canonical = encode_catalog_pack(&pack).expect("validated packs must encode");
        let decoded = decode_catalog_pack(&canonical).expect("canonical packs must decode");
        assert_eq!(decoded.catalog_hash(), pack.catalog_hash());
        assert_eq!(
            encode_catalog_pack(&decoded).expect("revalidated packs must encode"),
            canonical
        );
    }
});
