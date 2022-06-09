//! UUID helpers.

/// Generate a UUID version 7.
///
/// Implementation is based on [draft version 4](https://github.com/uuid6/uuid6-ietf-draft).
pub fn new_v7() -> uuid::Uuid {
    let time_now = std::time::SystemTime::now();
    let unix_duration = time_now.duration_since(std::time::UNIX_EPOCH).unwrap();

    let timestamp = unix_duration.as_millis();
    let random_value = rand::random::<[u8; 10]>();

    let mut bytes = [0u8; 16];
    bytes[0] = (timestamp >> 40) as u8;
    bytes[1] = (timestamp >> 32) as u8;
    bytes[2] = (timestamp >> 24) as u8;
    bytes[3] = (timestamp >> 16) as u8;
    bytes[4] = (timestamp >> 8) as u8;
    bytes[5] = timestamp as u8;
    bytes[6..16].copy_from_slice(&random_value);
    bytes[6] = (7 << 4) | (bytes[8] & 0x0f) as u8; // 4 bit version
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
        assert_eq!(uuid1.get_variant(), uuid::Variant::RFC4122);

        assert!(!uuid2.is_nil());
        assert_eq!(uuid2.get_version_num(), 7);
        assert_eq!(uuid2.get_variant(), uuid::Variant::RFC4122);

        assert_ne!(uuid1, uuid2);

        dbg!(uuid1, uuid2);
    }
}
