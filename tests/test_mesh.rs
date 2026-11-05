use amaranctl::mesh::{
    encode_onoff, encode_brightness, encode_cct,
    MeshSecurityContext, build_proxy_pdu, decrypt_proxy_pdu,
    decode_telink_status, opcode_bytes,
};

#[test]
fn test_encode_onoff() {
    let on = encode_onoff(true);
    assert_eq!(on[8], 0x01);
    assert_eq!(on[9], 0x8c);
    let sum: u16 = on[1..10].iter().map(|&b| b as u16).sum();
    assert_eq!(on[0], (sum & 0xff) as u8);

    let off = encode_onoff(false);
    assert_eq!(off[8], 0x00);
    assert_eq!(off[9], 0x8c);
    let sum_off: u16 = off[1..10].iter().map(|&b| b as u16).sum();
    assert_eq!(off[0], (sum_off & 0xff) as u8);
}

#[test]
fn test_encode_brightness() {
    // 100.0% -> 1000 (0x3E8)
    // 1000 & 3 = 0 -> (0 << 6) = 0
    // 1000 >> 2 = 250 (0xFA)
    let p = encode_brightness(100.0);
    assert_eq!(p[7], 0);
    assert_eq!(p[8], 0xfa);
    assert_eq!(p[9], 0x8f);
    let sum: u16 = p[1..10].iter().map(|&b| b as u16).sum();
    assert_eq!(p[0], (sum & 0xff) as u8);

    // 0.0% -> 0
    let p0 = encode_brightness(0.0);
    assert_eq!(p0[7], 0);
    assert_eq!(p0[8], 0);
    assert_eq!(p0[9], 0x8f);
    assert_eq!(p0[0], 0x8f);
}

#[test]
fn test_encode_cct() {
    let p = encode_cct(5600, 1000, 0);
    assert_eq!(p[9], 0x82);
    let sum: u16 = p[1..10].iter().map(|&b| b as u16).sum();
    assert_eq!(p[0], (sum & 0xff) as u8);
}

#[test]
fn test_mesh_security_context_and_proxy_pdu() {
    let net_key = hex::decode("58015449C4CEDAA2B9E1FA1BB580973F").unwrap();
    let app_key = hex::decode("FA1BB580973F58015449C4CEDAA2B9E1").unwrap();

    let sec = MeshSecurityContext::new(&net_key, &app_key).expect("context creation");
    assert_eq!(sec.enc_key.len(), 16);
    assert_eq!(sec.priv_key.len(), 16);

    let onoff_cmd = encode_onoff(true);
    let pdu = build_proxy_pdu(&sec, 12000001, 0x0001, 2, 0, 0x26, &onoff_cmd)
        .expect("build proxy pdu");
    
    // Check Proxy PDU starts with Network PDU header byte 0x00
    assert_eq!(pdu[0], 0x00);
    // Mesh Network PDU size check: 1 proxy header + 1 net header + 6 obf header + net payload + 4 mic
    assert!(pdu.len() >= 25);
}

#[test]
fn test_mesh_encryption_decryption_roundtrip() {
    let ctx = MeshSecurityContext::new(&[1u8; 16], &[2u8; 16]).unwrap();
    let pdu = build_proxy_pdu(&ctx, 100, 0x0001, 0x0002, 0, 0x26, &[0xAA, 0xBB]).unwrap();
    let (src, dst, decrypted) = decrypt_proxy_pdu(&ctx, 0, &pdu).unwrap();
    assert_eq!(src, 0x0001);
    assert_eq!(dst, 0x0002);
    assert_eq!(&decrypted[..1], &opcode_bytes(0x26)[..]);
    assert_eq!(&decrypted[1..], &[0xAA, 0xBB]);
}

#[test]
fn test_decode_telink_status_cct_packet() {
    // Valid 10-byte CCT status packet
    let mut packet = [0u8; 10];
    packet[9] = 0x02; // CCT command type
    // power on (bit 8 -> byte 1, bit 0)
    packet[1] = 0x01;
    // checksum
    let mut sum: u16 = 0;
    for i in 1..10 { sum += packet[i] as u16; }
    packet[0] = (sum & 0xff) as u8;

    let decoded = decode_telink_status(&packet).expect("valid decode");
    assert!(decoded.power);
}
