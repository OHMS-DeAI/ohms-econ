use ic_cdk_macros::*;
use candid::Principal;
use ic_cdk::api::caller;
use crate::domain::*;
use crate::services::{EstimationService, EscrowService, SettlementService, BalanceService, SubscriptionService, PaymentService};
use crate::services as svc;
use crate::services::{subscription, payment};
use ic_cdk::api::time;
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
    let pid = principal_id.unwrap_or_else(|| caller().to_text());
    BalanceService::get_balance(&pid)
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

// Admin role APIs
#[query]
fn is_admin() -> bool {
    let pid = caller().to_text();
    svc::is_admin(&pid)
}

#[query]
fn list_admins() -> Vec<String> {
    svc::list_admins()
}

#[update]
fn add_admin(principal_text: String) -> Result<(), String> {
    Guards::require_admin()?;
    svc::add_admin(principal_text);
    Ok(())
}

#[update]
fn remove_admin(principal_text: String) -> Result<(), String> {
    Guards::require_admin()?;
    svc::remove_admin(principal_text);
    Ok(())
}

// Subscription API
#[update]
async fn create_subscription(tier_name: String, auto_renew: bool) -> Result<subscription::UserSubscription, String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::create_subscription(pid, tier_name, auto_renew).await
}

#[query]
fn get_user_subscription(principal: Option<String>) -> Option<subscription::UserSubscription> {
    let pid = principal.unwrap_or_else(|| caller().to_text());
    SubscriptionService::get_user_subscription(&pid)
}

#[update]
async fn get_or_create_free_subscription() -> Result<subscription::UserSubscription, String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::get_or_create_free_subscription(pid).await
}

#[update]
async fn update_payment_status(status: subscription::PaymentStatus) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::update_payment_status(pid, status).await
}

#[update]
async fn validate_agent_creation_quota() -> Result<subscription::QuotaValidation, String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::validate_agent_creation_quota(&pid).await
}

#[update]
async fn validate_token_usage_quota(tokens_requested: u64) -> Result<subscription::QuotaValidation, String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::validate_token_usage_quota(&pid, tokens_requested).await
}

#[query]
fn get_user_usage(principal: Option<String>) -> Option<subscription::UsageMetrics> {
    let pid = principal.unwrap_or_else(|| caller().to_text());
    SubscriptionService::get_user_usage(&pid)
}

#[update]
async fn cancel_subscription() -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::cancel_subscription(pid).await
}

#[update]
async fn renew_subscription() -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    SubscriptionService::renew_subscription(pid).await
}

// Admin subscription APIs
#[query]
fn get_subscription_tiers() -> std::collections::HashMap<String, subscription::TierConfig> {
    Guards::require_admin()?;
    SubscriptionService::get_tier_configs()
}

#[query]
fn list_all_subscriptions() -> Vec<subscription::UserSubscription> {
    Guards::require_admin()?;
    SubscriptionService::list_all_subscriptions()
}

#[query]
fn get_subscription_stats() -> subscription::SubscriptionStats {
    Guards::require_admin()?;
    SubscriptionService::get_subscription_stats()
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
    let pid = principal_id.unwrap_or_else(|| caller().to_text());
    let max_limit = limit.unwrap_or(20).min(100);
    Ok(SettlementService::list_receipts(&pid, max_limit))
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
    BalanceService::deposit(caller().to_text(), amount)
}

#[update]
fn withdraw(amount: u64) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_amount(amount)?;
    BalanceService::withdraw(caller().to_text(), amount)
}

// Payment API
#[update]
async fn create_payment_request(subscription_tier: String) -> Result<payment::PaymentRequest, String> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    PaymentService::create_payment_request(pid, subscription_tier).await
}

#[update]
async fn process_subscription_payment(payment_request: payment::PaymentRequest) -> Result<payment::PaymentTransaction, String> {
    Guards::require_caller_authenticated()?;
    let from_principal = caller();
    PaymentService::process_icp_payment(payment_request, from_principal).await
}

#[update]
async fn verify_payment(transaction_id: String) -> Result<payment::PaymentVerification, String> {
    Guards::require_caller_authenticated()?;
    PaymentService::verify_payment(transaction_id).await
}

#[query]
fn get_payment_transaction(transaction_id: String) -> Option<payment::PaymentTransaction> {
    Guards::require_caller_authenticated()?;
    PaymentService::get_payment_transaction(transaction_id)
}

#[query]
fn list_user_payment_transactions(limit: Option<u32>) -> Vec<payment::PaymentTransaction> {
    Guards::require_caller_authenticated()?;
    let pid = caller().to_text();
    let max_limit = limit.unwrap_or(10).min(50);
    PaymentService::list_user_transactions(pid, max_limit)
}

#[query]
fn get_icp_usd_rate() -> Result<f64, String> {
    PaymentService::get_icp_usd_rate()
}

#[update]
async fn convert_usd_to_icp_e8s(amount_usd: u32) -> Result<u64, String> {
    PaymentService::usd_to_icp_e8s(amount_usd)
}

// Admin payment APIs
#[query]
fn get_payment_stats() -> payment::PaymentStats {
    Guards::require_admin()?;
    PaymentService::get_payment_stats()
}

#[query]
fn list_all_payment_transactions(limit: Option<u32>) -> Vec<payment::PaymentTransaction> {
    Guards::require_admin()?;
    let max_limit = limit.unwrap_or(50).min(200);
    svc::with_state(|state| {
        state.payment_transactions.as_ref()
            .map(|txs| {
                let mut transactions: Vec<payment::PaymentTransaction> = txs.values().cloned().collect();
                transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                transactions.into_iter().take(max_limit as usize).collect()
            })
            .unwrap_or_default()
    })
}