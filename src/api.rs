use ic_cdk_macros::*;
use crate::domain::*;
use crate::services::{EstimationService, EscrowService, SettlementService, BalanceService};
use crate::infra::{Guards, Metrics};

#[query]
fn estimate(job_spec: JobSpec) -> Result<CostQuote, String> {
    Guards::validate_job_spec(&job_spec)?;
    let quote = EstimationService::estimate_cost(job_spec)?;
    Metrics::increment_counter("estimates_requested_total");
    Ok(quote)
}

#[update]
async fn escrow(job_id: String, amount: u64) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_amount(amount)?;
    
    let escrow_id = EscrowService::create_escrow(job_id, amount).await?;
    Metrics::increment_counter("escrows_created_total");
    Ok(escrow_id)
}

#[update]
async fn settle(receipt: Receipt) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_receipt(&receipt)?;
    
    let settlement_id = SettlementService::settle_payment(receipt).await?;
    Metrics::increment_counter("settlements_processed_total");
    Ok(settlement_id)
}

#[query]
fn get_balance(principal_id: Option<String>) -> Result<Balance, String> {
    Guards::require_caller_authenticated()?;
    let caller_principal = principal_id.unwrap_or_else(|| "caller".to_string());
    BalanceService::get_balance(&caller_principal)
}

#[query]
fn policy() -> FeePolicy {
    BalanceService::get_fee_policy()
}

#[update]
fn update_policy(new_policy: FeePolicy) -> Result<(), String> {
    Guards::require_admin()?;
    BalanceService::update_fee_policy(new_policy)
}

#[query]
fn get_escrow(escrow_id: String) -> Result<EscrowAccount, String> {
    Guards::require_caller_authenticated()?;
    EscrowService::get_escrow(&escrow_id)
}

#[query]
fn get_receipt(receipt_id: String) -> Result<Receipt, String> {
    Guards::require_caller_authenticated()?;
    SettlementService::get_receipt(&receipt_id)
}

#[query]
fn list_receipts(principal_id: Option<String>, limit: Option<u32>) -> Result<Vec<Receipt>, String> {
    Guards::require_caller_authenticated()?;
    let caller_principal = principal_id.unwrap_or_else(|| "caller".to_string());
    let max_limit = limit.unwrap_or(20).min(100);
    Ok(SettlementService::list_receipts(&caller_principal, max_limit))
}

#[query]
fn health() -> EconHealth {
    BalanceService::get_health()
}

#[update]
fn refund_escrow(escrow_id: String) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    EscrowService::refund_escrow(escrow_id)
}

#[update]
fn deposit(amount: u64) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_amount(amount)?;
    BalanceService::deposit("caller".to_string(), amount)
}

#[update]
fn withdraw(amount: u64) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_amount(amount)?;
    BalanceService::withdraw("caller".to_string(), amount)
}