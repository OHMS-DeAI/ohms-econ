use crate::domain::*;
use crate::services::{with_state, with_state_mut, SubscriptionService};
use candid::{CandidType, Principal};
use ic_cdk::api::time;
// Simplified ICP ledger types for compatibility
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AccountIdentifier(pub Vec<u8>);

impl AccountIdentifier {
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        // Simplified hex parsing - in production would use proper hex crate
        let bytes = hex.as_bytes().to_vec();
        Ok(AccountIdentifier(bytes))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Memo(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Tokens {
    pub e8s: u64,
}

impl Tokens {
    pub fn from_e8s(e8s: u64) -> Self {
        Tokens { e8s }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct TransferArgs {
    pub to: AccountIdentifier,
    pub fee: Tokens,
    pub memo: Memo,
    pub from_subaccount: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
    pub amount: Tokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum TransferResult {
    Ok(u64),
    Err(TransferError),
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct TransferError {
    pub message: String,
}

const DEFAULT_FEE: Tokens = Tokens { e8s: 10_000 };
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Payment service for handling ICP payments through OISY wallet
pub struct PaymentService;

/// ICP Ledger canister ID
const ICP_LEDGER_CANISTER_ID: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";

/// Payment request for subscription
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct PaymentRequest {
    pub subscription_tier: String,
    pub amount_usd: u32,
    pub amount_icp_e8s: u64,
    pub user_principal: String,
    pub payment_memo: String,
}

/// Payment transaction record
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct PaymentTransaction {
    pub id: String,
    pub user_principal: String,
    pub subscription_tier: String,
    pub amount_usd: u32,
    pub amount_icp_e8s: u64,
    pub icp_block_index: Option<u64>,
    pub status: PaymentTransactionStatus,
    pub memo: String,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub error_message: Option<String>,
}

/// Payment transaction status
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, PartialEq)]
pub enum PaymentTransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
}

/// Payment verification result
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct PaymentVerification {
    pub verified: bool,
    pub transaction_id: String,
    pub block_index: Option<u64>,
    pub error_message: Option<String>,
}

impl PaymentService {
    /// Get current ICP/USD exchange rate (simplified - in production would use oracle)
    pub fn get_icp_usd_rate() -> Result<f64, String> {
        // Simplified rate - in production this would come from a price oracle
        // Using approximate rate of 1 ICP = $10 USD
        Ok(10.0)
    }

    /// Convert USD amount to ICP e8s (1 ICP = 100,000,000 e8s)
    pub fn usd_to_icp_e8s(amount_usd: u32) -> Result<u64, String> {
        let icp_rate = Self::get_icp_usd_rate()?;
        let icp_amount = amount_usd as f64 / icp_rate;
        let e8s_amount = (icp_amount * 100_000_000.0) as u64;
        Ok(e8s_amount)
    }

    /// Create a payment request for subscription
    pub async fn create_payment_request(
        user_principal: String,
        subscription_tier: String,
    ) -> Result<PaymentRequest, String> {
        // Get tier configuration
        let tier_configs = SubscriptionService::get_tier_configs();
        let tier_config = tier_configs.get(&subscription_tier)
            .ok_or("Invalid subscription tier")?;

        // Free tier doesn't require payment
        if tier_config.monthly_fee_usd == 0 {
            return Err("Free tier doesn't require payment".to_string());
        }

        // Convert USD to ICP e8s
        let amount_icp_e8s = Self::usd_to_icp_e8s(tier_config.monthly_fee_usd)?;

        // Create payment memo
        let payment_memo = format!("OHMS-{}-{}", subscription_tier.to_uppercase(), time());

        let payment_request = PaymentRequest {
            subscription_tier,
            amount_usd: tier_config.monthly_fee_usd,
            amount_icp_e8s,
            user_principal,
            payment_memo,
        };

        Ok(payment_request)
    }

    /// Process ICP payment through ledger
    pub async fn process_icp_payment(
        payment_request: PaymentRequest,
        from_principal: Principal,
    ) -> Result<PaymentTransaction, String> {
        let transaction_id = format!("tx_{}", time());
        
        // Create initial transaction record
        let mut transaction = PaymentTransaction {
            id: transaction_id.clone(),
            user_principal: payment_request.user_principal.clone(),
            subscription_tier: payment_request.subscription_tier.clone(),
            amount_usd: payment_request.amount_usd,
            amount_icp_e8s: payment_request.amount_icp_e8s,
            icp_block_index: None,
            status: PaymentTransactionStatus::Processing,
            memo: payment_request.payment_memo.clone(),
            created_at: time(),
            completed_at: None,
            error_message: None,
        };

        // Store transaction in pending state
        with_state_mut(|state| {
            if state.payment_transactions.is_none() {
                state.payment_transactions = Some(HashMap::new());
            }
            state.payment_transactions.as_mut().unwrap()
                .insert(transaction_id.clone(), transaction.clone());
        });

        // Get OHMS treasury account (in production this would be a proper treasury account)
        let treasury_account = AccountIdentifier::from_hex("2c4449a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8")
            .map_err(|e| format!("Invalid treasury account: {}", e))?;

        // Create transfer arguments
        let transfer_args = TransferArgs {
            memo: Memo(0), // Could encode payment info here
            amount: Tokens::from_e8s(payment_request.amount_icp_e8s),
            fee: DEFAULT_FEE,
            from_subaccount: None,
            to: treasury_account,
            created_at_time: None,
        };

        // Call ICP ledger for transfer
        let ledger_principal = Principal::from_text(ICP_LEDGER_CANISTER_ID)
            .map_err(|e| format!("Invalid ledger principal: {}", e))?;

        match ic_cdk::call::<(TransferArgs,), (TransferResult,)>(ledger_principal, "transfer", (transfer_args,)).await {
            Ok((transfer_result,)) => {
                match transfer_result {
                    TransferResult::Ok(block_index) => {
                        // Payment successful
                        transaction.status = PaymentTransactionStatus::Completed;
                        transaction.icp_block_index = Some(block_index);
                        transaction.completed_at = Some(time());

                        // Update subscription payment status
                        if let Err(e) = SubscriptionService::update_payment_status(
                            payment_request.user_principal.clone(),
                            crate::domain::PaymentStatus::Active,
                        ).await {
                            transaction.error_message = Some(format!("Failed to update subscription: {}", e));
                        }

                        // Store completed transaction
                        with_state_mut(|state| {
                            state.payment_transactions.as_mut().unwrap()
                                .insert(transaction_id, transaction.clone());
                        });

                        Ok(transaction)
                    }
                    TransferResult::Err(transfer_error) => {
                        // Payment failed
                        transaction.status = PaymentTransactionStatus::Failed;
                        transaction.error_message = Some(format!("Transfer failed: {:?}", transfer_error));
                        transaction.completed_at = Some(time());

                        with_state_mut(|state| {
                            state.payment_transactions.as_mut().unwrap()
                                .insert(transaction_id, transaction.clone());
                        });

                        Err(format!("Payment failed: {:?}", transfer_error))
                    }
                }
            }
            Err((rejection_code, rejection_message)) => {
                // Call to ledger failed
                transaction.status = PaymentTransactionStatus::Failed;
                transaction.error_message = Some(format!("Ledger call failed: {} - {}", rejection_code as u8, rejection_message));
                transaction.completed_at = Some(time());

                with_state_mut(|state| {
                    state.payment_transactions.as_mut().unwrap()
                        .insert(transaction_id, transaction.clone());
                });

                Err(format!("Ledger call failed: {}", rejection_message))
            }
        }
    }

    /// Verify payment transaction
    pub async fn verify_payment(transaction_id: String) -> Result<PaymentVerification, String> {
        let transaction = with_state(|state| {
            state.payment_transactions.as_ref()
                .and_then(|txs| txs.get(&transaction_id))
                .cloned()
        }).ok_or("Transaction not found")?;

        let verified = transaction.status == PaymentTransactionStatus::Completed 
            && transaction.icp_block_index.is_some();

        Ok(PaymentVerification {
            verified,
            transaction_id,
            block_index: transaction.icp_block_index,
            error_message: transaction.error_message,
        })
    }

    /// Get payment transaction
    pub fn get_payment_transaction(transaction_id: String) -> Option<PaymentTransaction> {
        with_state(|state| {
            state.payment_transactions.as_ref()
                .and_then(|txs| txs.get(&transaction_id))
                .cloned()
        })
    }

    /// List user payment transactions
    pub fn list_user_transactions(user_principal: String, limit: u32) -> Vec<PaymentTransaction> {
        with_state(|state| {
            state.payment_transactions.as_ref()
                .map(|txs| {
                    let mut transactions: Vec<PaymentTransaction> = txs.values()
                        .filter(|tx| tx.user_principal == user_principal)
                        .cloned()
                        .collect();
                    
                    // Sort by creation time (newest first)
                    transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                    
                    // Apply limit
                    transactions.into_iter().take(limit as usize).collect()
                })
                .unwrap_or_default()
        })
    }

    /// Get payment statistics (admin only)
    pub fn get_payment_stats() -> PaymentStats {
        with_state(|state| {
            let transactions = state.payment_transactions.as_ref()
                .map(|txs| txs.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            let mut stats = PaymentStats {
                total_transactions: transactions.len() as u32,
                completed_transactions: 0,
                failed_transactions: 0,
                pending_transactions: 0,
                total_revenue_usd: 0,
                total_revenue_icp_e8s: 0,
            };

            for transaction in transactions {
                match transaction.status {
                    PaymentTransactionStatus::Completed => {
                        stats.completed_transactions += 1;
                        stats.total_revenue_usd += transaction.amount_usd;
                        stats.total_revenue_icp_e8s += transaction.amount_icp_e8s;
                    }
                    PaymentTransactionStatus::Failed => stats.failed_transactions += 1,
                    PaymentTransactionStatus::Pending | PaymentTransactionStatus::Processing => {
                        stats.pending_transactions += 1;
                    }
                    _ => {}
                }
            }

            stats
        })
    }
}

/// Payment statistics for admin dashboard
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct PaymentStats {
    pub total_transactions: u32,
    pub completed_transactions: u32,
    pub failed_transactions: u32,
    pub pending_transactions: u32,
    pub total_revenue_usd: u32,
    pub total_revenue_icp_e8s: u64,
}