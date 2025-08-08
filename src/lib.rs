pub mod api;
pub mod domain;
pub mod services;
pub mod infra;

// Re-export main types and functions
pub use api::*;
pub use domain::*;
pub use services::*;
pub use infra::*;

use ic_cdk_macros::{init, pre_upgrade, post_upgrade};
use ic_cdk::api::stable::StableMemoryError;
use ic_cdk::api::caller;
use candid::Principal;

#[init]
fn init() {
    // Initialize default fee policy and grant the installer as admin
    let installer = caller();
    services::with_state_mut(|state| {
        if state.fee_policy.last_updated == 0 {
            state.fee_policy = domain::FeePolicy::default();
        }
        if let Some(text) = principal_to_text(&installer) {
            if !state.admins.iter().any(|p| p == &text) {
                state.admins.push(text);
            }
        }
        state.state_version = 1;
    });
}

#[pre_upgrade]
fn pre_upgrade() {
    let state = services::get_state_clone();
    match ic_cdk::storage::stable_save((state,)) {
        Ok(()) => {}
        Err(e) => ic_cdk::trap(&format!("Failed to save stable state: {:?}", e)),
    }
}

#[post_upgrade]
fn post_upgrade() {
    match ic_cdk::storage::stable_restore::<(services::EconState,)>() {
        Ok((mut restored,)) => {
            // Migrate if needed
            if restored.state_version == 0 {
                restored.state_version = 1;
                if restored.receipt_to_settlement.is_empty() {
                    // Rebuild mapping from settlements where possible
                    // Note: keys in settlements are settlement_id, we can map receipt_id via stored entries
                    // This is best-effort and safe to be empty if not derivable
                }
            }
            services::set_state(restored);
        }
        Err(_) => {
            // Fresh install or corrupted state; keep defaults
        }
    }
}

fn principal_to_text(p: &Principal) -> Option<String> {
    if *p == Principal::anonymous() { None } else { Some(p.to_text()) }
}