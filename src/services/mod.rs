use crate::domain::*;
use ic_cdk::api::time;
use std::collections::HashMap;
use std::cell::RefCell;

pub mod estimation;
pub mod escrow;
pub mod settlement;
pub mod balance;

pub use estimation::EstimationService;
pub use escrow::EscrowService;
pub use settlement::SettlementService;
pub use balance::BalanceService;

thread_local! {
    static STATE: RefCell<EconState> = RefCell::new(EconState::default());
}

#[derive(Debug, Default)]
pub struct EconState {
    pub escrows: HashMap<String, EscrowAccount>,
    pub receipts: HashMap<String, Receipt>,
    pub balances: HashMap<String, Balance>,
    pub settlements: HashMap<String, SettlementEntry>,
    pub fee_policy: FeePolicy,
    pub metrics: EconMetrics,
}

#[derive(Debug, Default)]
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