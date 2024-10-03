use std::collections::{HashMap, HashSet};

use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct MutationTestIxn<'a> {
    pub name: &'static str,
    pub ixn: Instruction,
    pub signers: Vec<&'a Keypair>,
    pub skip_mutating: HashSet<(Pubkey, MutationType)>,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum MutationType {
    Writable,
    Signer,
    Account,
    AccountSpecific(Pubkey),
}

pub fn mutate<'a>(
    ixn: &Instruction,
    signers: &[&'a Keypair],
    kps: &[&'a Keypair],
    account_names: &HashMap<Pubkey, String>,
    skip_mutating: &HashSet<(Pubkey, MutationType)>,
) -> Vec<(Instruction, Vec<&'a Keypair>, String)> {
    let mut v = Vec::new();

    // mutate is_writable
    for (i, ac) in ixn.accounts.iter().enumerate() {
        if skip_mutating.contains(&(ac.pubkey, MutationType::Writable)) {
            continue;
        }
        if ac.is_writable {
            let ac_name = lookup_account_name(&ac.pubkey, account_names);
            let mutation_name = format!("{} -> is_writable = {}", ac_name, !ac.is_writable);
            let mut ixn_ = ixn.clone();
            ixn_.accounts[i].is_writable = !ac.is_writable;
            v.push((ixn_, signers.to_vec(), mutation_name));
        }
    }

    // mutate is_signer
    for (i, ac) in ixn.accounts.iter().enumerate() {
        if skip_mutating.contains(&(ac.pubkey, MutationType::Signer)) {
            continue;
        }
        if ac.is_signer {
            let ac_name = lookup_account_name(&ac.pubkey, account_names);
            let mutation_name = format!("{} -> is_signer = {}", ac_name, !ac.is_signer);
            let mut ixn_ = ixn.clone();
            ixn_.accounts[i].is_signer = !ac.is_signer;
            // remove from signers vec
            let mut signers_ = signers.to_vec();
            signers_.retain(|s| s.pubkey() != ac.pubkey);
            v.push((ixn_, signers_.to_vec(), mutation_name));
        }
    }

    // mutate account
    for (i, ac) in ixn.accounts.iter().enumerate() {
        // For example: we need to skip mutating super_admin in init.
        // Whichever account is passed as super_admin in init, that becomes the super_admin.
        // Same goes for admin in init.
        if skip_mutating.contains(&(ac.pubkey, MutationType::Account)) {
            continue;
        }

        for kp in kps {
            if ac.pubkey == kp.pubkey() {
                continue;
            }

            // For example: we need to skip changing admin -> owner in
            // cancel_deposit because it's not an error to calll it as owner
            if skip_mutating.contains(&(ac.pubkey, MutationType::AccountSpecific(kp.pubkey()))) {
                continue;
            }

            let ac_name = lookup_account_name(&ac.pubkey, account_names);
            let ac_new_name = lookup_account_name(&kp.pubkey(), account_names);
            let mutation_name = format!("{} -> {}", ac_name, ac_new_name);

            let mut ixn_ = ixn.clone();
            ixn_.accounts[i].pubkey = kp.pubkey();

            let mut signers_ = signers.to_vec();
            for s in &mut signers_ {
                if s.pubkey() == ac.pubkey {
                    *s = kp;
                }
            }
            v.push((ixn_, signers_.to_vec(), mutation_name));
        }
    }

    return v;
}

fn lookup_account_name(pubkey: &Pubkey, account_names: &HashMap<Pubkey, String>) -> String {
    account_names
        .get(pubkey)
        .cloned()
        .unwrap_or_else(|| format!("Unknown Account {}", pubkey))
}
