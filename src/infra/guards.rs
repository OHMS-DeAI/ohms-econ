use ic_cdk::api::caller;
use candid::Principal;
use crate::domain::*;

pub struct Guards;

impl Guards {
    pub fn require_caller_authenticated() -> Result<(), String> {
        let caller = caller();
        if caller == Principal::anonymous() {
            return Err("Authentication required".to_string());
        }
        Ok(())
    }
    
    pub fn require_admin() -> Result<(), String> {
        Self::require_caller_authenticated()?;
        // TODO: Implement proper admin check
        Ok(())
    }
    
    pub fn validate_amount(amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err("Amount must be greater than zero".to_string());
        }
        
        if amount > 1_000_000_000_000 { // 1M tokens max
            return Err("Amount too large".to_string());
        }
        
        Ok(())
    }
    
    pub fn validate_job_spec(job_spec: &JobSpec) -> Result<(), String> {
        if job_spec.job_id.is_empty() {
            return Err("Job ID cannot be empty".to_string());
        }
        
        if job_spec.model_id.is_empty() {
            return Err("Model ID cannot be empty".to_string());
        }
        
        if job_spec.estimated_tokens == 0 {
            return Err("Estimated tokens must be greater than zero".to_string());
        }
        
        Ok(())
    }
    
    pub fn validate_receipt(receipt: &Receipt) -> Result<(), String> {
        if receipt.receipt_id.is_empty() {
            return Err("Receipt ID cannot be empty".to_string());
        }
        
        if receipt.job_id.is_empty() {
            return Err("Job ID cannot be empty".to_string());
        }
        
        if receipt.escrow_id.is_empty() {
            return Err("Escrow ID cannot be empty".to_string());
        }
        
        if receipt.actual_cost == 0 {
            return Err("Actual cost must be greater than zero".to_string());
        }
        
        // Validate fees breakdown
        let expected_total = receipt.fees_breakdown.base_amount 
            + receipt.fees_breakdown.protocol_fee 
            + receipt.fees_breakdown.agent_fee;
            
        if receipt.fees_breakdown.total_amount != expected_total {
            return Err("Fees breakdown does not match total amount".to_string());
        }
        
        Ok(())
    }
}