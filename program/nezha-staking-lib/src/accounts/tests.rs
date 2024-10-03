use std::str::FromStr;

use solana_program::pubkey::Pubkey;

#[test]
fn guard_against_accidental_changes() {
    // We are using a random hardcoded value here instead of Pubkey::new_unique() because
    // version upgrades could change the value of Pubkey::new_unique()
    let program_id = Pubkey::from_str("stkt5YJMm5gVBRaFER6QNhkfteSZFU64MeR4BaiH8cL").unwrap();
    let owner = Pubkey::from_str("HBUuCX45eWrmE4G7nEFABrHcjn9znVqy17VH4WzAyoXD").unwrap();
    let epoch_index = 123;
    let winners_page = 2;
    let tier = 2;

    let latest_epoch = super::latest_epoch(&program_id);
    assert_eq!(
        latest_epoch.pubkey.to_string(),
        "CNJHuGV5Aj45Dpaq4Jv2jNUJULXakAg27EZsP2corKg2"
    );

    let epoch = super::epoch(&program_id, epoch_index);
    assert_eq!(epoch.pubkey.to_string(), "DjQnT7iuo5ngigRuPF56Hux1Jf4yFzphkFNzCpSvV5pH");

    let epoch_winners_page = super::epoch_winners_page(&program_id, epoch_index, winners_page);
    assert_eq!(
        epoch_winners_page.pubkey.to_string(),
        "XSdc69woGvVBpa4NvsPvTEGH3DsMP7HY17C5P4SyNbb"
    );

    let epoch_winners_meta = super::epoch_winners_meta(&program_id, epoch_index);
    assert_eq!(
        epoch_winners_meta.pubkey.to_string(),
        "5mdnFhkrzaL8SwJkTwcFJpZh2saDKPZXKvaD5oPqErM1"
    );

    let stake = super::stake(&program_id, &owner);
    assert_eq!(stake.pubkey.to_string(), "JA5TefPnfDXHnQAqmwXnniRuj5ppYrEZQkvsgSYhmnYZ");

    let deposit_attempt = super::stake_update_request(&program_id, &owner);
    assert_eq!(
        deposit_attempt.pubkey.to_string(),
        "3BqacPNrRSRPsDWPkenLWToPJ9cMFzScP2wptXoJcVDo"
    );

    let vault_authority = super::vault_authority(&program_id);
    assert_eq!(
        vault_authority.pubkey.to_string(),
        "8iB5Akg7baTHH9pqeb1JHz46gt5BdZs9sq5ggL7rv5VA"
    );

    let deposit_vault = super::deposit_vault(&program_id);
    assert_eq!(
        deposit_vault.pubkey.to_string(),
        "i8eRsxxfdvx64VXMeY3uLVuTw6xyG1vpChZrrTCQ2d3"
    );

    let treasury_vault = super::treasury_vault(&program_id);
    assert_eq!(
        treasury_vault.pubkey.to_string(),
        "DphUy8HBiL9cpFpAq7Dy8f9SdEpnke4HJJTLGzoy7PFA"
    );

    let insurance_vault = super::insurance_vault(&program_id);
    assert_eq!(
        insurance_vault.pubkey.to_string(),
        "4N4xRWYBnPncdEhnGA2jRoaHXzDK9fGjgo57c2oKfG8S"
    );

    let prize_vault = super::prize_vault(&program_id, tier);
    assert_eq!(
        prize_vault.pubkey.to_string(),
        "8Yt7zHdSahi9SHBQZpkx6MdjzQ4PLuBFo2LXwmLNt34a"
    );

    let pending_deposit_vault = super::pending_deposit_vault(&program_id);
    assert_eq!(
        pending_deposit_vault.pubkey.to_string(),
        "8DXa6MSAWmT4mcRSL8FNrbmr7uT49ybNaeSYyiUkuXDK"
    );
}
