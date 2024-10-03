use std::str::FromStr;

use solana_program::pubkey::Pubkey;

#[test]
fn guard_against_accidental_changes() {
    // We are using a random hardcoded value here instead of Pubkey::new_unique() because
    // version upgrades could change the value of Pubkey::new_unique()
    let program_id = Pubkey::from_str("stkt5YJMm5gVBRaFER6QNhkfteSZFU64MeR4BaiH8cL").unwrap();
    let epoch_index = 123;

    let program_state = super::nezha_vrf_program_state(&program_id);
    assert_eq!(
        program_state.pubkey.to_string(),
        "2V1y7p1LxEZrYSwWGBdwZYBR3pvzDdhQXKXDK2uabtsr"
    );

    let vrf_request = super::nezha_vrf_request(&program_id, epoch_index);
    assert_eq!(
        vrf_request.pubkey.to_string(),
        "itaeiXNtDvYxN7zLcKtQ7K1LGA7rURqG1wRUvXwywji"
    );

    let switchboard_authority = super::switchboard_authority(&program_id);
    assert_eq!(
        switchboard_authority.pubkey.to_string(),
        "HNebv6iVuBuKYYnJLMP1uDcG2478u2eD68AE9SVkwjoy"
    );

    let switchboard_vrf_lite = super::switchboard_vrf_lite(&program_id);
    assert_eq!(
        switchboard_vrf_lite.pubkey.to_string(),
        "8JhZ3t7eXo3BxDbVHzb7rdKds4rv2mDYLmqLvZrEyXpD"
    );
}
