use crate::domain::*;
use crate::services::{with_state, with_state_mut, EscrowService};
use ic_cdk::api::time;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct SettlementService;

impl SettlementService {
    pub async fn settle_payment(receipt: Receipt) -> Result<String, String> {
        let now = time();
        let settlement_id = Self::generate_settlement_id(&receipt.receipt_id);
        
        // Check for idempotency
        if Self::is_duplicate_settlement(&receipt.receipt_id) {
            return Err("Receipt already settled".to_string());
        }
        
        // Validate escrow exists and is active
        let escrow = EscrowService::get_escrow(&receipt.escrow_id)?;
        if !matches!(escrow.status, EscrowStatus::Active) {
            return Err("Escrow is not active".to_string());
        }
        
        if escrow.amount < receipt.actual_cost {
            return Err("Insufficient escrow amount for settlement".to_string());
        }
        
        // Release funds to agent
        EscrowService::release_escrow(receipt.escrow_id.clone(), receipt.agent_id.clone(), receipt.actual_cost)?;
        
        // Record settlement
        let settlement_entry = SettlementEntry {
            receipt_id: receipt.receipt_id.clone(),
            processed_at: now,
            amount: receipt.actual_cost,
            status: SettlementStatus::Completed,
            idempotency_key: Self::generate_idempotency_key(&receipt),
        };
        
        let receipt_cost = receipt.actual_cost;
        let protocol_fee = receipt.fees_breakdown.protocol_fee;
        
        with_state_mut(|state| {
            state.receipts.insert(receipt.receipt_id.clone(), receipt);
            state.settlements.insert(settlement_id.clone(), settlement_entry);
            
            // Update metrics
            state.metrics.total_settlements += 1;
            state.metrics.total_volume += receipt_cost;
            state.metrics.protocol_fees_collected += protocol_fee;
            state.metrics.last_activity = now;
        });
        
        Ok(settlement_id)
    }
    
    pub fn get_receipt(receipt_id: &str) -> Result<Receipt, String> {
        with_state(|state| {
            state.receipts
                .get(receipt_id)
                .cloned()
                .ok_or_else(|| format!("Receipt not found: {}", receipt_id))
        })
    }
    
    pub fn list_receipts(principal_id: &str, limit: u32) -> Vec<Receipt> {
        with_state(|state| {
            state.receipts
                .values()
                .filter(|receipt| {
                    // In real implementation, check if principal owns this receipt
                    true
                })
                .take(limit as usize)
                .cloned()
                .collect()
        })
    }
    
    pub fn calculate_fees(base_amount: u64, fee_policy: &FeePolicy) -> FeesBreakdown {
        let protocol_fee = ((base_amount as f64) * (fee_policy.protocol_fee_percentage as f64 / 100.0)) as u64;
        let agent_fee = ((base_amount as f64) * (fee_policy.agent_fee_percentage as f64 / 100.0)) as u64;
        let total_amount = base_amount + protocol_fee + agent_fee;
        
        FeesBreakdown {
            base_amount,
            protocol_fee,
            agent_fee,
            total_amount,
        }
    }
    
    pub fn verify_settlement_integrity(receipt_id: &str) -> Result<bool, String> {
        with_state(|state| {
            if let Some(receipt) = state.receipts.get(receipt_id) {
                if let Some(settlement) = state.settlements.get(receipt_id) {
                    // Verify amounts match
                    let amounts_match = receipt.actual_cost == settlement.amount;
                    
                    // Verify status consistency
                    let status_consistent = matches!(
                        (&receipt.settlement_status, &settlement.status),
                        (SettlementStatus::Completed, SettlementStatus::Completed) |
                        (SettlementStatus::Failed, SettlementStatus::Failed)
                    );
                    
                    Ok(amounts_match && status_consistent)
                } else {
                    Err("Settlement record not found".to_string())
                }
            } else {
                Err("Receipt not found".to_string())
            }
        })
    }
    
    fn is_duplicate_settlement(receipt_id: &str) -> bool {
        with_state(|state| {
            state.settlements.contains_key(receipt_id)
        })
    }
    
    fn generate_settlement_id(receipt_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(receipt_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("settlement_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
    
    fn generate_idempotency_key(receipt: &Receipt) -> String {
        let mut hasher = Sha256::new();
        hasher.update(receipt.receipt_id.as_bytes());
        hasher.update(receipt.job_id.as_bytes());
        hasher.update(receipt.escrow_id.as_bytes());
        hasher.update(receipt.actual_cost.to_be_bytes());
        let hash = hasher.finalize();
        general_purpose::STANDARD.encode(&hash[..16])
    }
}