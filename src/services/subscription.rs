use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;

/// Subscription service for managing user subscriptions and quotas
pub struct SubscriptionService;

/// Subscription tier configuration with limits and pricing
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct TierConfig {
    pub name: String,
    pub monthly_fee_usd: u32,
    pub max_agents: u32,
    pub monthly_agent_creations: u32,
    pub token_limit: u64,
    pub inference_rate: InferenceRate,
    pub features: Vec<String>,
}

/// Inference rate priority levels
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum InferenceRate {
    Standard,
    Priority,
    Premium,
}

/// User subscription with current tier and usage tracking
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct UserSubscription {
    pub principal_id: String,
    pub tier: TierConfig,
    pub started_at: u64,
    pub expires_at: u64,
    pub auto_renew: bool,
    pub current_usage: UsageMetrics,
    pub payment_status: PaymentStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Usage metrics for quota tracking
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct UsageMetrics {
    pub agents_created_this_month: u32,
    pub tokens_used_this_month: u64,
    pub inferences_this_month: u32,
    pub last_reset_date: u64,
}

/// Payment status tracking
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum PaymentStatus {
    Active,
    Pending,
    Failed,
    Cancelled,
}

/// Quota validation result
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaValidation {
    pub allowed: bool,
    pub reason: Option<String>,
    pub remaining_quota: Option<QuotaRemaining>,
}

/// Remaining quota information
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaRemaining {
    pub agents_remaining: u32,
    pub tokens_remaining: u64,
    pub inferences_remaining: u32,
}

impl SubscriptionService {
    /// Get predefined subscription tiers
    pub fn get_tier_configs() -> HashMap<String, TierConfig> {
        let mut tiers = HashMap::new();
        
        tiers.insert("free".to_string(), TierConfig {
            name: "Free".to_string(),
            monthly_fee_usd: 0,
            max_agents: 1,
            monthly_agent_creations: 3,
            token_limit: 10_000,
            inference_rate: InferenceRate::Standard,
            features: vec![
                "1 concurrent agent".to_string(),
                "3 agent creations per month".to_string(),
                "10K tokens per month".to_string(),
                "Standard inference priority".to_string(),
                "Community support".to_string(),
            ],
        });

        tiers.insert("basic".to_string(), TierConfig {
            name: "Basic".to_string(),
            monthly_fee_usd: 29,
            max_agents: 5,
            monthly_agent_creations: 10,
            token_limit: 100_000,
            inference_rate: InferenceRate::Standard,
            features: vec![
                "5 concurrent agents".to_string(),
                "10 agent creations per month".to_string(),
                "100K tokens per month".to_string(),
                "Standard inference priority".to_string(),
            ],
        });

        tiers.insert("pro".to_string(), TierConfig {
            name: "Pro".to_string(),
            monthly_fee_usd: 99,
            max_agents: 25,
            monthly_agent_creations: 50,
            token_limit: 500_000,
            inference_rate: InferenceRate::Priority,
            features: vec![
                "25 concurrent agents".to_string(),
                "50 agent creations per month".to_string(),
                "500K tokens per month".to_string(),
                "Priority inference".to_string(),
                "Advanced analytics".to_string(),
            ],
        });

        tiers.insert("enterprise".to_string(), TierConfig {
            name: "Enterprise".to_string(),
            monthly_fee_usd: 299,
            max_agents: 100,
            monthly_agent_creations: 200,
            token_limit: 2_000_000,
            inference_rate: InferenceRate::Premium,
            features: vec![
                "100 concurrent agents".to_string(),
                "200 agent creations per month".to_string(),
                "2M tokens per month".to_string(),
                "Premium inference priority".to_string(),
                "Advanced analytics".to_string(),
                "Priority support".to_string(),
                "Custom integrations".to_string(),
            ],
        });

        tiers
    }

    /// Create a new subscription for a user
    pub async fn create_subscription(
        principal_id: String,
        tier_name: String,
        auto_renew: bool,
    ) -> Result<UserSubscription, String> {
        let tier_configs = Self::get_tier_configs();
        let tier_config = tier_configs.get(&tier_name)
            .ok_or("Invalid subscription tier")?;

        // Check if user already has an active subscription
        if Self::get_user_subscription(&principal_id).is_some() {
            return Err("User already has an active subscription".to_string());
        }

        // For free tier, auto-renew is always true and payment status is always Active
        let actual_auto_renew = if tier_name == "free" { true } else { auto_renew };
        let payment_status = if tier_name == "free" { 
            PaymentStatus::Active 
        } else { 
            PaymentStatus::Pending 
        };

        let now = time();
        let expires_at = now + (30 * 24 * 60 * 60 * 1_000_000_000); // 30 days in nanoseconds

        let subscription = UserSubscription {
            principal_id: principal_id.clone(),
            tier: tier_config.clone(),
            started_at: now,
            expires_at,
            auto_renew: actual_auto_renew,
            current_usage: UsageMetrics {
                agents_created_this_month: 0,
                tokens_used_this_month: 0,
                inferences_this_month: 0,
                last_reset_date: now,
            },
            payment_status,
            created_at: now,
            updated_at: now,
        };

        // Store subscription
        with_state_mut(|state| {
            state.subscriptions.insert(principal_id, subscription.clone());
        });

        Ok(subscription)
    }

    /// Get user subscription
    pub fn get_user_subscription(principal_id: &str) -> Option<UserSubscription> {
        with_state(|state| {
            state.subscriptions.get(principal_id).cloned()
        })
    }

    /// Get or create free tier subscription for user
    pub async fn get_or_create_free_subscription(principal_id: String) -> Result<UserSubscription, String> {
        // Check if user already has a subscription
        if let Some(subscription) = Self::get_user_subscription(&principal_id) {
            return Ok(subscription);
        }

        // Create free tier subscription for new user
        Self::create_subscription(principal_id, "free".to_string(), true).await
    }

    /// Update subscription payment status
    pub async fn update_payment_status(
        principal_id: String,
        status: PaymentStatus,
    ) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(subscription) = state.subscriptions.get_mut(&principal_id) {
                subscription.payment_status = status;
                subscription.updated_at = time();
            }
        });
        Ok(())
    }

    /// Validate quota for agent creation
    pub async fn validate_agent_creation_quota(principal_id: &str) -> Result<QuotaValidation, String> {
        let subscription = Self::get_user_subscription(principal_id)
            .ok_or("No active subscription found")?;

        // Check if subscription is active
        if subscription.payment_status != PaymentStatus::Active {
            return Ok(QuotaValidation {
                allowed: false,
                reason: Some("Subscription payment required".to_string()),
                remaining_quota: None,
            });
        }

        // Check if subscription is expired
        if time() > subscription.expires_at {
            return Ok(QuotaValidation {
                allowed: false,
                reason: Some("Subscription expired".to_string()),
                remaining_quota: None,
            });
        }

        // Reset monthly usage if needed
        let mut updated_subscription = subscription.clone();
        Self::reset_monthly_usage_if_needed(&mut updated_subscription);

        // Check agent creation quota
        if updated_subscription.current_usage.agents_created_this_month >= updated_subscription.tier.monthly_agent_creations {
            return Ok(QuotaValidation {
                allowed: false,
                reason: Some("Monthly agent creation limit reached".to_string()),
                remaining_quota: Some(QuotaRemaining {
                    agents_remaining: 0,
                    tokens_remaining: updated_subscription.tier.token_limit.saturating_sub(updated_subscription.current_usage.tokens_used_this_month),
                    inferences_remaining: 0,
                }),
            });
        }

        // Update usage and store
        updated_subscription.current_usage.agents_created_this_month += 1;
        updated_subscription.updated_at = time();
        
        with_state_mut(|state| {
            state.subscriptions.insert(principal_id.to_string(), updated_subscription.clone());
        });

        Ok(QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: updated_subscription.tier.monthly_agent_creations.saturating_sub(updated_subscription.current_usage.agents_created_this_month),
                tokens_remaining: updated_subscription.tier.token_limit.saturating_sub(updated_subscription.current_usage.tokens_used_this_month),
                inferences_remaining: 0,
            }),
        })
    }

    /// Validate quota for token usage
    pub async fn validate_token_usage_quota(
        principal_id: &str,
        tokens_requested: u64,
    ) -> Result<QuotaValidation, String> {
        let subscription = Self::get_user_subscription(principal_id)
            .ok_or("No active subscription found")?;

        // Check if subscription is active
        if subscription.payment_status != PaymentStatus::Active {
            return Ok(QuotaValidation {
                allowed: false,
                reason: Some("Subscription payment required".to_string()),
                remaining_quota: None,
            });
        }

        // Reset monthly usage if needed
        let mut updated_subscription = subscription.clone();
        Self::reset_monthly_usage_if_needed(&mut updated_subscription);

        // Check token quota
        let remaining_tokens = updated_subscription.tier.token_limit.saturating_sub(updated_subscription.current_usage.tokens_used_this_month);
        
        if tokens_requested > remaining_tokens {
            return Ok(QuotaValidation {
                allowed: false,
                reason: Some("Insufficient token quota".to_string()),
                remaining_quota: Some(QuotaRemaining {
                    agents_remaining: updated_subscription.tier.monthly_agent_creations.saturating_sub(updated_subscription.current_usage.agents_created_this_month),
                    tokens_remaining: remaining_tokens,
                    inferences_remaining: 0,
                }),
            });
        }

        // Update usage and store
        updated_subscription.current_usage.tokens_used_this_month += tokens_requested;
        updated_subscription.updated_at = time();
        
        with_state_mut(|state| {
            state.subscriptions.insert(principal_id.to_string(), updated_subscription.clone());
        });

        Ok(QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: updated_subscription.tier.monthly_agent_creations.saturating_sub(updated_subscription.current_usage.agents_created_this_month),
                tokens_remaining: updated_subscription.tier.token_limit.saturating_sub(updated_subscription.current_usage.tokens_used_this_month),
                inferences_remaining: 0,
            }),
        })
    }

    /// Get user usage metrics
    pub fn get_user_usage(principal_id: &str) -> Option<UsageMetrics> {
        Self::get_user_subscription(principal_id)
            .map(|sub| sub.current_usage)
    }

    /// List all subscriptions (admin only)
    pub fn list_all_subscriptions() -> Vec<UserSubscription> {
        with_state(|state| {
            state.subscriptions.values().cloned().collect()
        })
    }

    /// Cancel subscription
    pub async fn cancel_subscription(principal_id: String) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(subscription) = state.subscriptions.get_mut(&principal_id) {
                subscription.auto_renew = false;
                subscription.updated_at = time();
            }
        });
        Ok(())
    }

    /// Renew subscription
    pub async fn renew_subscription(principal_id: String) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(subscription) = state.subscriptions.get_mut(&principal_id) {
                let now = time();
                subscription.expires_at = now + (30 * 24 * 60 * 60 * 1_000_000_000); // 30 days
                subscription.payment_status = PaymentStatus::Active;
                subscription.updated_at = now;
                
                // Reset monthly usage
                subscription.current_usage = UsageMetrics {
                    agents_created_this_month: 0,
                    tokens_used_this_month: 0,
                    inferences_this_month: 0,
                    last_reset_date: now,
                };
            }
        });
        Ok(())
    }

    /// Reset monthly usage if a new month has started
    fn reset_monthly_usage_if_needed(subscription: &mut UserSubscription) {
        let now = time();
        let last_reset = subscription.current_usage.last_reset_date;
        
        // Check if we're in a new month (simple check: 30 days passed)
        if now - last_reset > 30 * 24 * 60 * 60 * 1_000_000_000 {
            subscription.current_usage = UsageMetrics {
                agents_created_this_month: 0,
                tokens_used_this_month: 0,
                inferences_this_month: 0,
                last_reset_date: now,
            };
        }
    }

    /// Get subscription statistics (admin only)
    pub fn get_subscription_stats() -> SubscriptionStats {
        let subscriptions = Self::list_all_subscriptions();
        
        let mut stats = SubscriptionStats {
            total_subscriptions: subscriptions.len() as u32,
            active_subscriptions: 0,
            expired_subscriptions: 0,
            pending_payments: 0,
            tier_distribution: HashMap::new(),
            total_monthly_revenue_usd: 0,
        };

        let now = time();
        
        for subscription in subscriptions {
            // Count by status
            match subscription.payment_status {
                PaymentStatus::Active => stats.active_subscriptions += 1,
                PaymentStatus::Pending => stats.pending_payments += 1,
                _ => {},
            }

            // Count expired
            if now > subscription.expires_at {
                stats.expired_subscriptions += 1;
            }

            // Count by tier
            let tier_name = subscription.tier.name.clone();
            *stats.tier_distribution.entry(tier_name).or_insert(0) += 1;

            // Calculate revenue (only for active subscriptions)
            if subscription.payment_status == PaymentStatus::Active {
                stats.total_monthly_revenue_usd += subscription.tier.monthly_fee_usd;
            }
        }

        stats
    }
}

/// Subscription statistics for admin dashboard
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct SubscriptionStats {
    pub total_subscriptions: u32,
    pub active_subscriptions: u32,
    pub expired_subscriptions: u32,
    pub pending_payments: u32,
    pub tier_distribution: HashMap<String, u32>,
    pub total_monthly_revenue_usd: u32,
}
