use crate::domain::*;
use ic_cdk::api::time;
use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;
use std::cell::RefCell;

pub mod estimation;
pub mod escrow;
pub mod settlement;
pub mod balance;
pub mod subscription;
pub mod payment;

pub use estimation::EstimationService;
pub use escrow::EscrowService;
pub use settlement::SettlementService;
pub use balance::BalanceService;
pub use subscription::SubscriptionService;
pub use payment::PaymentService;

thread_local! {
    static STATE: RefCell<EconState> = RefCell::new(EconState::default());
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, CandidType)]
pub struct EconState {
    pub escrows: HashMap<String, EscrowAccount>,
    pub receipts: HashMap<String, Receipt>,
    pub balances: HashMap<String, Balance>,
    pub settlements: HashMap<String, SettlementEntry>,
    // Map receipt_id -> settlement_id for O(1) integrity checks
    pub receipt_to_settlement: HashMap<String, String>,
    pub fee_policy: FeePolicy,
    pub metrics: EconMetrics,
    // Governance/admins
    pub admins: Vec<String>,
    // Versioning for future stable state migrations
    pub state_version: u32,
    // Subscription map
    pub subscriptions: HashMap<String, Subscription>,
    // Payment transactions
    pub payment_transactions: Option<HashMap<String, payment::PaymentTransaction>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, CandidType)]
pub struct EconMetrics {
    pub total_volume: u64,
    pub protocol_fees_collected: u64,
    pub total_estimates: u64,
    pub total_settlements: u64,
    pub last_activity: u64,
}

pub fn with_state<R>(f: impl FnOnce(&EconState) -> R) -> R {
    STATE.with(|s| f(&*s.borrow()))
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut EconState) -> R) -> R {
    STATE.with(|s| f(&mut *s.borrow_mut()))
}

pub fn get_state_clone() -> EconState {
    with_state(|s| s.clone())
}

pub fn set_state(new_state: EconState) {
    STATE.with(|s| *s.borrow_mut() = new_state)
}

// Admin helpers
pub fn is_admin(principal_text: &str) -> bool {
    with_state(|state| state.admins.iter().any(|p| p == principal_text))
}

pub fn add_admin(principal_text: String) {
    with_state_mut(|state| {
        if !state.admins.iter().any(|p| p == &principal_text) {
            state.admins.push(principal_text);
            state.metrics.last_activity = time();
        }
    });
}

pub fn list_admins() -> Vec<String> {
    with_state(|state| state.admins.clone())
}

pub fn remove_admin(principal_text: String) {
    with_state_mut(|state| {
        state.admins.retain(|p| p != &principal_text);
        state.metrics.last_activity = time();
    });
}