use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct EstimationService;

impl EstimationService {
    const BASE_COST_PER_TOKEN: u64 = 100; // 0.0001 tokens per output token
    const COMPUTE_CYCLE_COST: u64 = 10;   // 0.00001 tokens per compute cycle
    
    pub fn estimate_cost(job_spec: JobSpec) -> Result<CostQuote, String> {
        let now = time();
        
        with_state_mut(|state| {
            // Calculate base cost
            let token_cost = job_spec.estimated_tokens as u64 * Self::BASE_COST_PER_TOKEN;
            let compute_cost = job_spec.estimated_compute_cycles * Self::COMPUTE_CYCLE_COST;
            let base_cost = token_cost + compute_cost;
            
            // Apply priority multiplier
            let priority_multiplier = Self::get_priority_multiplier(&job_spec.priority, &state.fee_policy);
            let adjusted_cost = (base_cost as f64 * priority_multiplier as f64) as u64;
            
            // Calculate protocol fee
            let protocol_fee = ((adjusted_cost as f64) * (state.fee_policy.protocol_fee_percentage as f64 / 100.0)) as u64;
            let total_cost = adjusted_cost + protocol_fee;
            
            // Ensure minimum fee
            let final_cost = total_cost.max(state.fee_policy.minimum_fee);
            
            let quote_id = Self::generate_quote_id(&job_spec.job_id);
            let quote = CostQuote {
                job_id: job_spec.job_id,
                estimated_cost: final_cost,
                base_cost,
                priority_multiplier,
                protocol_fee,
                quote_expires_at: now + 15 * 60 * 1_000_000_000, // 15 minutes
                quote_id,
            };
            
            state.metrics.total_estimates += 1;
            state.metrics.last_activity = now;
            
            Ok(quote)
        })
    }
    
    pub fn validate_quote(quote: &CostQuote) -> Result<(), String> {
        let now = time();
        
        if quote.quote_expires_at < now {
            return Err("Quote has expired".to_string());
        }
        
        if quote.estimated_cost < quote.base_cost {
            return Err("Invalid quote: estimated cost less than base cost".to_string());
        }
        
        Ok(())
    }
    
    pub fn estimate_variance(actual_cost: u64, estimated_cost: u64) -> f32 {
        if estimated_cost == 0 {
            return 0.0;
        }
        
        let variance = ((actual_cost as f64 - estimated_cost as f64) / estimated_cost as f64).abs();
        (variance * 100.0) as f32 // Return as percentage
    }
    
    pub fn update_estimation_model(actual_costs: &[(JobSpec, u64)]) -> Result<(), String> {
        // Mock implementation for estimation model updates
        // In real implementation, this would use machine learning to improve estimates
        with_state_mut(|state| {
            let total_jobs = actual_costs.len();
            if total_jobs > 0 {
                let average_variance = actual_costs
                    .iter()
                    .map(|(job_spec, actual_cost)| {
                        let estimated_cost = job_spec.estimated_tokens as u64 * Self::BASE_COST_PER_TOKEN;
                        Self::estimate_variance(*actual_cost, estimated_cost)
                    })
                    .sum::<f32>() / total_jobs as f32;
                
                // Log average variance for monitoring
                log::info!("Estimation model update: average variance = {:.2}%", average_variance);
            }
            
            Ok(())
        })
    }
    
    fn get_priority_multiplier(priority: &JobPriority, policy: &FeePolicy) -> f32 {
        let priority_key = match priority {
            JobPriority::Low => "Low",
            JobPriority::Normal => "Normal", 
            JobPriority::High => "High",
            JobPriority::Critical => "Critical",
        };
        
        policy.priority_multipliers
            .get(priority_key)
            .copied()
            .unwrap_or(1.0)
    }
    
    fn generate_quote_id(job_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(job_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("quote_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
}