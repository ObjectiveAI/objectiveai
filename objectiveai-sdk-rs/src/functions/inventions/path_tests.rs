use super::path::*;

#[test]
fn roundtrip_u128() {
    let path: Vec<u8> = vec![0, 1, 254, 100];
    let encoded = path_to_u128(&path).unwrap();
    let decoded: Vec<u8> = u128_to_path(encoded).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn roundtrip_b62() {
    let path: Vec<u16> = vec![3, 200, 0, 42];
    let b62 = path_to_b62(&path).unwrap();
    let decoded: Vec<u16> = b62_to_path(&b62).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn empty_path() {
    let path: Vec<u8> = vec![];
    let encoded = path_to_u128(&path).unwrap();
    assert_eq!(encoded, 0);
    let decoded: Vec<u8> = u128_to_path(encoded).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn single_element() {
    let path: Vec<u8> = vec![0];
    let encoded = path_to_u128(&path).unwrap();
    assert_eq!(encoded, 1);
    let decoded: Vec<u8> = u128_to_path(encoded).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn different_lengths_differ() {
    let a = path_to_u128(&[0u8]).unwrap();
    let b = path_to_u128(&[0u8, 0]).unwrap();
    assert_ne!(a, b);
}

#[test]
fn max_length() {
    let path: Vec<u8> = vec![254; 16];
    let encoded = path_to_u128(&path).unwrap();
    let decoded: Vec<u8> = u128_to_path(encoded).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn too_long() {
    let path: Vec<u8> = vec![0; 17];
    assert!(path_to_u128(&path).is_err());
}

#[test]
fn value_too_large() {
    let path: Vec<u16> = vec![255];
    assert!(path_to_u128(&path).is_err());
}
