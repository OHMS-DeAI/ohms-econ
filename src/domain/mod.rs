use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct JobSpec {
    pub job_id: String,
    pub model_id: String,
    pub estimated_tokens: u32,
    pub estimated_compute_cycles: u64,
    pub priority: JobPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CostQuote {
    pub job_id: String,
    pub estimated_cost: u64,
    pub base_cost: u64,
    pub priority_multiplier: f32,
    pub protocol_fee: u64,
    pub quote_expires_at: u64,
    pub quote_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct EscrowAccount {
    pub escrow_id: String,
    pub job_id: String,
    pub principal_id: String,
    pub amount: u64,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum EscrowStatus {
    Pending,
    Active,
    Released,
    Refunded,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Receipt {
    pub receipt_id: String,
    pub job_id: String,
    pub escrow_id: String,
    pub agent_id: String,
    pub actual_cost: u64,
    pub fees_breakdown: FeesBreakdown,
    pub settlement_status: SettlementStatus,
    pub created_at: u64,
    pub settled_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct FeesBreakdown {
    pub base_amount: u64,
    pub protocol_fee: u64,
    pub agent_fee: u64,
    pub total_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum SettlementStatus {
    Pending,
    Completed,
    Failed,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct FeePolicy {
    pub protocol_fee_percentage: f32,
    pub agent_fee_percentage: f32,
    pub minimum_fee: u64,
    pub priority_multipliers: HashMap<String, f32>,
    pub last_updated: u64,
}

impl Default for FeePolicy {
    fn default() -> Self {
        let mut priority_multipliers = HashMap::new();
        priority_multipliers.insert("Low".to_string(), 0.8);
        priority_multipliers.insert("Normal".to_string(), 1.0);
        priority_multipliers.insert("High".to_string(), 1.5);
        priority_multipliers.insert("Critical".to_string(), 2.0);

        Self {
            protocol_fee_percentage: 3.0,
            agent_fee_percentage: 7.0,
            minimum_fee: 1000, // 0.001 tokens
            priority_multipliers,
            last_updated: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Balance {
    pub principal_id: String,
    pub available_balance: u64,
    pub escrowed_balance: u64,
    pub total_earnings: u64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct EconHealth {
    pub total_escrows: u32,
    pub active_escrows: u32,
    pub total_receipts: u32,
    pub pending_settlements: u32,
    pub total_volume: u64,
    pub protocol_fees_collected: u64,
    pub average_job_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementEntry {
    pub receipt_id: String,
    pub processed_at: u64,
    pub amount: u64,
    pub status: SettlementStatus,
    pub idempotency_key: String,
}