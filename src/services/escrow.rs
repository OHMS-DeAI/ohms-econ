use crate::domain::*;
use crate::services::{with_state, with_state_mut, BalanceService};
use ic_cdk::api::{time, caller};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct EscrowService;

impl EscrowService {
    const ESCROW_TTL: u64 = 24 * 60 * 60 * 1_000_000_000; // 24 hours in nanoseconds
    
    pub async fn create_escrow(job_id: String, amount: u64) -> Result<String, String> {
        let now = time();
        let escrow_id = Self::generate_escrow_id(&job_id);
        let principal_id = caller().to_text();
        
        // Check if user has sufficient balance
        let balance = BalanceService::get_balance(&principal_id)?;
        if balance.available_balance < amount {
            return Err("Insufficient balance".to_string());
        }
        
        let escrow = EscrowAccount {
            escrow_id: escrow_id.clone(),
            job_id,
            principal_id: principal_id.clone(),
            amount,
            status: EscrowStatus::Active,
            created_at: now,
            expires_at: now + Self::ESCROW_TTL,
        };
        
        with_state_mut(|state| {
            // Move funds from available to escrowed
            if let Some(user_balance) = state.balances.get_mut(&principal_id) {
                user_balance.available_balance -= amount;
                user_balance.escrowed_balance += amount;
                user_balance.last_updated = now;
            }
            
            state.escrows.insert(escrow_id.clone(), escrow);
        });
        
        Ok(escrow_id)
    }
    
    pub fn get_escrow(escrow_id: &str) -> Result<EscrowAccount, String> {
        with_state(|state| {
            state.escrows
                .get(escrow_id)
                .cloned()
                .ok_or_else(|| format!("Escrow not found: {}", escrow_id))
        })
    }
    
    pub fn release_escrow(escrow_id: String, recipient: String, amount: u64) -> Result<(), String> {
        let now = time();
        
        with_state_mut(|state| {
            if let Some(escrow) = state.escrows.get_mut(&escrow_id) {
                if !matches!(escrow.status, EscrowStatus::Active) {
                    return Err("Escrow is not active".to_string());
                }
                
                if escrow.amount < amount {
                    return Err("Insufficient escrow amount".to_string());
                }
                
                // Release funds to recipient
                let recipient_balance = state.balances.entry(recipient.clone()).or_insert_with(|| Balance {
                    principal_id: recipient,
                    available_balance: 0,
                    escrowed_balance: 0,
                    total_earnings: 0,
                    last_updated: now,
                });
                
                recipient_balance.available_balance += amount;
                recipient_balance.total_earnings += amount;
                recipient_balance.last_updated = now;
                
                // Update escrow holder's balance
                if let Some(holder_balance) = state.balances.get_mut(&escrow.principal_id) {
                    holder_balance.escrowed_balance -= amount;
                    holder_balance.last_updated = now;
                }
                
                // Mark escrow as released
                escrow.status = EscrowStatus::Released;
                
                Ok(())
            } else {
                Err("Escrow not found".to_string())
            }
        })
    }
    
    pub fn refund_escrow(escrow_id: String) -> Result<(), String> {
        let now = time();
        
        with_state_mut(|state| {
            if let Some(escrow) = state.escrows.get_mut(&escrow_id) {
                if !matches!(escrow.status, EscrowStatus::Active) {
                    return Err("Escrow is not active".to_string());
                }
                
                // Refund to original holder
                if let Some(holder_balance) = state.balances.get_mut(&escrow.principal_id) {
                    holder_balance.available_balance += escrow.amount;
                    holder_balance.escrowed_balance -= escrow.amount;
                    holder_balance.last_updated = now;
                }
                
                escrow.status = EscrowStatus::Refunded;
                
                Ok(())
            } else {
                Err("Escrow not found".to_string())
            }
        })
    }
    
    pub fn cleanup_expired_escrows() -> u32 {
        let now = time();
        
        with_state_mut(|state| {
            let mut expired_count = 0;
            
            let expired_ids: Vec<String> = state.escrows
                .iter()
                .filter(|(_, escrow)| {
                    matches!(escrow.status, EscrowStatus::Active) && escrow.expires_at < now
                })
                .map(|(id, _)| id.clone())
                .collect();
            
            for escrow_id in expired_ids {
                if Self::refund_escrow_internal(escrow_id, state).is_ok() {
                    expired_count += 1;
                }
            }
            
            expired_count
        })
    }
    
    fn refund_escrow_internal(escrow_id: String, state: &mut crate::services::EconState) -> Result<(), String> {
        let now = time();
        
        if let Some(escrow) = state.escrows.get_mut(&escrow_id) {
            if let Some(holder_balance) = state.balances.get_mut(&escrow.principal_id) {
                holder_balance.available_balance += escrow.amount;
                holder_balance.escrowed_balance -= escrow.amount;
                holder_balance.last_updated = now;
            }
            
            escrow.status = EscrowStatus::Expired;
        }
        
        Ok(())
    }
    
    fn generate_escrow_id(job_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(job_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("escrow_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
}