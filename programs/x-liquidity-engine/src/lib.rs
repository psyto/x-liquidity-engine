use anchor_lang::prelude::*;

declare_id!("5eKPz3P7vBT1RhMUoYadmHB4KaNwjSoaUPaNvEzjcuKx");

#[program]
pub mod x_liquidity_engine {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

// ============================================================================
// ENUMS AND TYPES
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum DexType {
    Raydium,
    Orca,
    Meteora,
    Unknown,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum PositionStatus {
    Active,
    Paused,
    Closed,
    Liquidated,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum ExecutionStatus {
    Pending,
    Executed,
    Failed,
    Rejected,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum PaymentStatus {
    Pending,
    Verified,
    Settled,
    Failed,
    Refunded,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum PaymentCurrency {
    SOL,
    USDC,
    USDT,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum StrategyType {
    Conservative,
    Balanced,
    Aggressive,
    Custom,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum RebalanceFrequency {
    OnSignal,
    Daily,
    Weekly,
    Monthly,
    Manual,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum ComplianceMode {
    Basic,
    Enhanced,
    Full,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum AuditEventType {
    PositionCreated,
    PositionClosed,
    Rebalanced,
    FeesCollected,
    PaymentReceived,
    PolicyViolation,
    HumanApprovalRequired,
    HumanApprovalGranted,
}

// ============================================================================
// ACCOUNT STRUCTURES
// ============================================================================

/// Tracks individual LP positions managed by the protocol
#[account]
pub struct LiquidityPosition {
    // Ownership & Identity
    pub owner: Pubkey,
    pub position_bump: u8,
    
    // Token Pair
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    
    // DEX Integration
    pub dex: DexType,
    pub pool_address: Pubkey,
    pub position_nft: Option<Pubkey>,
    
    // Price Range (Concentrated Liquidity)
    pub current_tick_lower: i32,
    pub current_tick_upper: i32,
    pub current_price_lower: u128,
    pub current_price_upper: u128,
    
    // Position Metrics
    pub liquidity_amount: u128,
    pub total_fees_earned_a: u64,
    pub total_fees_earned_b: u64,
    pub total_value_locked: u64,
    
    // Rebalancing History
    pub last_rebalance_slot: u64,
    pub last_rebalance_timestamp: i64,
    pub rebalance_count: u32,
    
    // Performance Metrics
    pub total_return_percentage: i16,
    pub apy_estimate: u16,
    
    // Status & Configuration
    pub status: PositionStatus,
    pub auto_rebalance_enabled: bool,
    pub min_rebalance_interval: u32,
    
    // Policy Controls (Compliance)
    pub max_position_size: u64,
    pub max_single_trade: u64,
    pub allowed_dex_programs: Vec<Pubkey>,
    
    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

/// Stores AI decision metadata for compliance and auditability
#[account]
pub struct RebalanceDecision {
    // Position Reference
    pub position: Pubkey,
    pub decision_bump: u8,
    
    // New Price Range
    pub new_tick_lower: i32,
    pub new_tick_upper: i32,
    pub new_price_lower: u128,
    pub new_price_upper: u128,
    
    // AI Model Information (Explainability)
    pub ai_model_version: String,
    pub ai_model_hash: [u8; 32],
    pub prediction_confidence: u16,
    
    // Input Data (For Audit Trail)
    pub market_sentiment_score: i16,
    pub volatility_metric: u16,
    pub whale_activity_score: u16,
    pub on_chain_indicators: Vec<u64>,
    
    // Decision Rationale
    pub decision_reason: String,
    pub risk_assessment: RiskLevel,
    
    // Execution Details
    pub execution_status: ExecutionStatus,
    pub execution_tx_signature: Option<String>,
    pub execution_slippage: Option<u16>,
    
    // Compliance & Audit
    pub requires_human_approval: bool,
    pub human_approver: Option<Pubkey>,
    pub approval_timestamp: Option<i64>,
    
    // Timestamps
    pub created_at: i64,
    pub executed_at: Option<i64>,
}

/// Tracks x402 protocol payments for API access
#[account]
pub struct X402Payment {
    // Payment Identity
    pub payment_id: [u8; 32],
    pub payment_bump: u8,
    
    // Payer Information
    pub payer: Pubkey,
    pub payer_wallet: Pubkey,
    
    // Payment Details
    pub amount: u64,
    pub currency: PaymentCurrency,
    pub payment_status: PaymentStatus,
    
    // x402 Protocol Details
    pub facilitator: Pubkey,
    pub facilitator_signature: Option<[u8; 64]>,
    pub payment_tx_signature: Option<String>,
    
    // API Access Details
    pub api_endpoint: String,
    pub api_version: String,
    pub access_granted: bool,
    pub access_expires_at: Option<i64>,
    
    // Timestamps
    pub requested_at: i64,
    pub verified_at: Option<i64>,
    pub settled_at: Option<i64>,
}

/// Global protocol configuration and parameters
#[account]
pub struct ProtocolConfig {
    // Authority
    pub authority: Pubkey,
    pub config_bump: u8,
    
    // Fee Structure
    pub performance_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub fee_recipient: Pubkey,
    
    // x402 Configuration
    pub x402_facilitator: Option<Pubkey>,
    pub x402_min_payment: u64,
    pub x402_api_base_url: String,
    
    // Rebalancing Parameters
    pub min_rebalance_interval: u32,
    pub max_rebalance_frequency: u32,
    pub default_slippage_tolerance_bps: u16,
    
    // Risk Management
    pub max_position_size: u64,
    pub max_single_trade_size: u64,
    pub require_human_approval_threshold: u64,
    
    // AI Model Configuration
    pub default_ai_model_version: String,
    pub ai_model_registry: Vec<Pubkey>,
    
    // Compliance
    pub audit_log_enabled: bool,
    pub compliance_mode: ComplianceMode,
    
    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

/// User-defined strategy parameters and preferences
#[account]
pub struct UserStrategy {
    // Ownership
    pub user: Pubkey,
    pub strategy_bump: u8,
    
    // Strategy Configuration
    pub strategy_name: String,
    pub strategy_type: StrategyType,
    pub risk_tolerance: RiskTolerance,
    
    // Rebalancing Preferences
    pub auto_rebalance_enabled: bool,
    pub rebalance_frequency: RebalanceFrequency,
    pub price_range_width: u16,
    
    // Risk Limits
    pub max_position_size: Option<u64>,
    pub max_single_trade: Option<u64>,
    pub max_slippage_bps: Option<u16>,
    
    // Token Preferences
    pub preferred_tokens: Vec<Pubkey>,
    pub blacklisted_tokens: Vec<Pubkey>,
    
    // AI Model Selection
    pub preferred_ai_model: Option<String>,
    pub require_human_approval: bool,
    
    // Timestamps
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub enum RiskTolerance {
    Low,
    Medium,
    High,
}

/// Immutable audit log for compliance and regulatory requirements
#[account]
pub struct AuditLog {
    // Log Identity
    pub log_id: [u8; 32],
    pub log_bump: u8,
    
    // Event Information
    pub event_type: AuditEventType,
    pub position: Option<Pubkey>,
    pub user: Pubkey,
    
    // Event Data
    pub event_data: Vec<u8>,
    pub event_hash: [u8; 32],
    
    // AI Decision Context (if applicable)
    pub ai_model_version: Option<String>,
    pub decision_rationale: Option<String>,
    pub prediction_scores: Option<Vec<u16>>,
    
    // Compliance Metadata
    pub regulatory_jurisdiction: Option<String>,
    
    // Timestamps
    pub created_at: i64,
    pub slot: u64,
}

// ============================================================================
// INSTRUCTION CONTEXTS
// ============================================================================

#[derive(Accounts)]
pub struct Initialize {}
