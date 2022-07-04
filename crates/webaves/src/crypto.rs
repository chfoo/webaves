//! Cryptography tools.

/// Normalizes a hash algorithm name.
///
/// Changes to lowercase. Removes the hyphen from SHA-1 and SHA-2 names.
pub fn normalize_hash_name<S: Into<String>>(name: S) -> String {
    let mut name = name.into();
    name.make_ascii_lowercase();

    match name.as_str() {
        "sha-1" => {
            name.remove(3);
        }
        "sha-224" => {
            name.remove(3);
        }
        "sha-256" => {
            name.remove(3);
        }
        "sha-384" => {
            name.remove(3);
        }
        "sha-512" => {
            name.remove(3);
        }
        _ => {}
    }

    name
}

/// Returns a hash function from the given name.
pub fn get_hash_function_by_name<S: Into<String>>(name: S) -> Option<Box<dyn digest::DynDigest>> {
    let name = normalize_hash_name(name);

    match name.as_str() {
        "md5" => Some(Box::new(md5::Md5::default())),
        "sha1" => Some(Box::new(sha1::Sha1::default())),
        "sha224" => Some(Box::new(sha2::Sha224::default())),
        "sha256" => Some(Box::new(sha2::Sha256::default())),
        "sha384" => Some(Box::new(sha2::Sha384::default())),
        "sha512" => Some(Box::new(sha2::Sha512::default())),
        "sha3-224" => Some(Box::new(sha3::Sha3_224::default())),
        "sha3-256" => Some(Box::new(sha3::Sha3_256::default())),
        "sha3-384" => Some(Box::new(sha3::Sha3_384::default())),
        "sha3-512" => Some(Box::new(sha3::Sha3_512::default())),
        "blake2s" => Some(Box::new(blake2::Blake2s256::default())),
        "blake2b" => Some(Box::new(blake2::Blake2b512::default())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hash_name() {
        assert_eq!(normalize_hash_name("Sha-1"), "sha1");
        assert_eq!(normalize_hash_name("SHA-256"), "sha256");
        assert_eq!(normalize_hash_name("BLAKE2s"), "blake2s");
    }
}
