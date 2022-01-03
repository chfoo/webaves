/// UUID helpers.

/// Generate a UUID version 7.
///
/// Implementation is based on [draft version 2](https://github.com/uuid6/uuid6-ietf-draft).
pub fn new_v7() -> uuid::Uuid {
    let time_now = std::time::SystemTime::now();
    let unix_duration = time_now.duration_since(std::time::UNIX_EPOCH).unwrap();

    let timestamp = unix_duration.as_secs();
    let nanoseconds =
        (unix_duration.subsec_nanos() as f32 / 1_000_000_000.0 * 0xff_ffff as f32) as u32;
    let random_value = rand::random::<[u8; 8]>();

    let mut bytes = [0u8; 16];
    bytes[0] = (timestamp >> 28) as u8;
    bytes[1] = (timestamp >> 20) as u8;
    bytes[2] = (timestamp >> 12) as u8;
    bytes[3] = (timestamp >> 4) as u8;
    bytes[4] = (((timestamp & 0x0f) as u8) << 4) | ((nanoseconds >> 20) & 0x0f) as u8;
    bytes[5] = (nanoseconds >> 12) as u8;
    bytes[6] = (7 << 4) | ((nanoseconds >> 8) & 0x0f) as u8; // 4 bit version
    bytes[7] = nanoseconds as u8;

    bytes[8..16].copy_from_slice(&random_value);
    bytes[8] = (0b10 << 6) | (bytes[8] & 0b11_1111); // variant

    uuid::Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_uuidv7() {
        let uuid1 = new_v7();
        let uuid2 = new_v7();

        assert!(!uuid1.is_nil());
        assert_eq!(uuid1.get_version_num(), 7);
        assert_eq!(uuid1.get_variant(), Some(uuid::Variant::RFC4122));

        assert!(!uuid2.is_nil());
        assert_eq!(uuid2.get_version_num(), 7);
        assert_eq!(uuid2.get_variant(), Some(uuid::Variant::RFC4122));

        assert_ne!(uuid1, uuid2);

        dbg!(uuid1, uuid2);
    }
}
