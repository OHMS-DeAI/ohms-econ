use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;

pub struct BalanceService;

impl BalanceService {
    pub fn get_balance(principal_id: &str) -> Result<Balance, String> {
        with_state(|state| {
            if let Some(balance) = state.balances.get(principal_id) {
                Ok(balance.clone())
            } else {
                // Create default balance for new user
                Ok(Balance {
                    principal_id: principal_id.to_string(),
                    available_balance: 0,
                    escrowed_balance: 0,
                    total_earnings: 0,
                    last_updated: time(),
                })
            }
        })
    }
    
    pub fn deposit(principal_id: String, amount: u64) -> Result<(), String> {
        let now = time();
        
        with_state_mut(|state| {
            let balance = state.balances.entry(principal_id.clone()).or_insert_with(|| Balance {
                principal_id: principal_id.clone(),
                available_balance: 0,
                escrowed_balance: 0,
                total_earnings: 0,
                last_updated: now,
            });
            
            balance.available_balance += amount;
            balance.last_updated = now;
        });
        
        Ok(())
    }
    
    pub fn withdraw(principal_id: String, amount: u64) -> Result<(), String> {
        let now = time();
        
        with_state_mut(|state| {
            if let Some(balance) = state.balances.get_mut(&principal_id) {
                if balance.available_balance < amount {
                    return Err("Insufficient balance".to_string());
                }
                
                balance.available_balance -= amount;
                balance.last_updated = now;
                Ok(())
            } else {
                Err("Balance not found".to_string())
            }
        })
    }
    
    pub fn get_fee_policy() -> FeePolicy {
        with_state(|state| state.fee_policy.clone())
    }
    
    pub fn update_fee_policy(new_policy: FeePolicy) -> Result<(), String> {
        let now = time();
        
        with_state_mut(|state| {
            let mut updated_policy = new_policy;
            updated_policy.last_updated = now;
            state.fee_policy = updated_policy;
        });
        
        Ok(())
    }
    
    pub fn get_health() -> EconHealth {
        with_state(|state| {
            let total_escrows = state.escrows.len() as u32;
            let active_escrows = state.escrows
                .values()
                .filter(|escrow| matches!(escrow.status, EscrowStatus::Active))
                .count() as u32;
            
            let total_receipts = state.receipts.len() as u32;
            let pending_settlements = state.receipts
                .values()
                .filter(|receipt| matches!(receipt.settlement_status, SettlementStatus::Pending))
                .count() as u32;
            
            let average_job_cost = if total_receipts > 0 {
                state.metrics.total_volume as f64 / total_receipts as f64
            } else {
                0.0
            };
            
            EconHealth {
                total_escrows,
                active_escrows,
                total_receipts,
                pending_settlements,
                total_volume: state.metrics.total_volume,
                protocol_fees_collected: state.metrics.protocol_fees_collected,
                average_job_cost,
            }
        })
    }
}