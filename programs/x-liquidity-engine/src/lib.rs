use anchor_lang::prelude::*;

declare_id!("5eKPz3P7vBT1RhMUoYadmHB4KaNwjSoaUPaNvEzjcuKx");

#[program]
pub mod x_liquidity_engine {
    use super::*;

    /// Initialize the protocol configuration
    pub fn initialize_protocol_config(
        ctx: Context<InitializeProtocolConfig>,
        performance_fee_bps: u16,
        protocol_fee_bps: u16,
        x402_min_payment: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        config.authority = ctx.accounts.authority.key();
        config.config_bump = ctx.bumps.config;
        config.performance_fee_bps = performance_fee_bps;
        config.protocol_fee_bps = protocol_fee_bps;
        config.fee_recipient = ctx.accounts.fee_recipient.key();
        config.x402_min_payment = x402_min_payment;
        config.x402_api_base_url = "https://api.x-liquidity-engine.com".to_string();
        config.min_rebalance_interval = 3600; // 1 hour default
        config.max_rebalance_frequency = 24; // Max 24 per day
        config.default_slippage_tolerance_bps = 50; // 0.5% default
        config.max_position_size = 1_000_000_000_000; // $1M default (scaled)
        config.max_single_trade_size = 100_000_000_000; // $100K default (scaled)
        config.require_human_approval_threshold = 500_000_000_000; // $500K threshold
        config.default_ai_model_version = "v1.0.0".to_string();
        config.audit_log_enabled = true;
        config.compliance_mode = ComplianceMode::Enhanced;
        config.created_at = clock.unix_timestamp;
        config.updated_at = clock.unix_timestamp;

        msg!("Protocol config initialized by: {}", ctx.accounts.authority.key());
        Ok(())
    }

    /// Create a new liquidity position
    pub fn create_liquidity_position(
        ctx: Context<CreateLiquidityPosition>,
        _position_index: u8,
        token_a: Pubkey,
        token_b: Pubkey,
        tick_lower: i32,
        tick_upper: i32,
        price_lower: u128,
        price_upper: u128,
        max_position_size: u64,
        max_single_trade: u64,
    ) -> Result<()> {
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Validate price range
        require!(tick_lower < tick_upper, XLiquidityEngineError::InvalidPriceRange);
        require!(price_lower < price_upper, XLiquidityEngineError::InvalidPriceRange);

        // Validate against protocol limits
        let config = &ctx.accounts.config;
        require!(
            max_position_size <= config.max_position_size,
            XLiquidityEngineError::ExceedsMaxPositionSize
        );
        require!(
            max_single_trade <= config.max_single_trade_size,
            XLiquidityEngineError::ExceedsMaxTradeSize
        );

        position.owner = ctx.accounts.owner.key();
        position.position_bump = ctx.bumps.position;
        position.token_a = token_a;
        position.token_b = token_b;
        position.token_a_vault = ctx.accounts.token_a_vault.key();
        position.token_b_vault = ctx.accounts.token_b_vault.key();
        position.dex = DexType::Raydium; // Default to Raydium
        position.pool_address = ctx.accounts.pool.key();
        position.current_tick_lower = tick_lower;
        position.current_tick_upper = tick_upper;
        position.current_price_lower = price_lower;
        position.current_price_upper = price_upper;
        position.liquidity_amount = 0;
        position.total_fees_earned_a = 0;
        position.total_fees_earned_b = 0;
        position.total_value_locked = 0;
        position.last_rebalance_slot = 0;
        position.last_rebalance_timestamp = 0;
        position.rebalance_count = 0;
        position.total_return_percentage = 0;
        position.apy_estimate = 0;
        position.status = PositionStatus::Active;
        position.auto_rebalance_enabled = true;
        position.min_rebalance_interval = config.min_rebalance_interval;
        position.max_position_size = max_position_size;
        position.max_single_trade = max_single_trade;
        position.allowed_dex_programs = vec![ctx.accounts.pool.key()];
        position.created_at = clock.unix_timestamp;
        position.updated_at = clock.unix_timestamp;

        // Create audit log
        create_audit_log_internal(
            &ctx.accounts.audit_log,
            AuditEventType::PositionCreated,
            Some(position.key()),
            ctx.accounts.owner.key(),
            &[],
            clock,
        )?;

        msg!("Liquidity position created: {}", position.key());
        Ok(())
    }

    /// Create a rebalancing decision based on AI prediction
    pub fn create_rebalance_decision(
        ctx: Context<CreateRebalanceDecision>,
        _position_index: u8,
        _decision_index: u32,
        new_tick_lower: i32,
        new_tick_upper: i32,
        new_price_lower: u128,
        new_price_upper: u128,
        ai_model_version: String,
        ai_model_hash: [u8; 32],
        prediction_confidence: u16,
        market_sentiment_score: i16,
        volatility_metric: u16,
        whale_activity_score: u16,
        decision_reason: String,
    ) -> Result<()> {
        let decision = &mut ctx.accounts.decision;
        let position = &ctx.accounts.position;
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate position is active
        require!(
            position.status == PositionStatus::Active,
            XLiquidityEngineError::PositionNotActive
        );

        // Validate rebalance interval
        require!(
            clock.unix_timestamp - position.last_rebalance_timestamp >= position.min_rebalance_interval as i64,
            XLiquidityEngineError::RebalanceTooFrequent
        );

        // Validate price range
        require!(new_tick_lower < new_tick_upper, XLiquidityEngineError::InvalidPriceRange);
        require!(new_price_lower < new_price_upper, XLiquidityEngineError::InvalidPriceRange);

        // Determine risk level and if human approval is needed
        let risk_assessment = assess_risk(
            prediction_confidence,
            market_sentiment_score,
            volatility_metric,
        );
        let requires_human_approval = risk_assessment == RiskLevel::Critical
            || risk_assessment == RiskLevel::High
            || position.total_value_locked >= config.require_human_approval_threshold;

        decision.position = position.key();
        decision.decision_bump = ctx.bumps.decision;
        decision.new_tick_lower = new_tick_lower;
        decision.new_tick_upper = new_tick_upper;
        decision.new_price_lower = new_price_lower;
        decision.new_price_upper = new_price_upper;
        decision.ai_model_version = ai_model_version;
        decision.ai_model_hash = ai_model_hash;
        decision.prediction_confidence = prediction_confidence;
        decision.market_sentiment_score = market_sentiment_score;
        decision.volatility_metric = volatility_metric;
        decision.whale_activity_score = whale_activity_score;
        decision.on_chain_indicators = vec![];
        decision.decision_reason = decision_reason;
        decision.risk_assessment = risk_assessment;
        decision.execution_status = if requires_human_approval {
            ExecutionStatus::Pending
        } else {
            ExecutionStatus::Pending
        };
        decision.execution_tx_signature = None;
        decision.execution_slippage = None;
        decision.requires_human_approval = requires_human_approval;
        decision.human_approver = None;
        decision.approval_timestamp = None;
        decision.created_at = clock.unix_timestamp;
        decision.executed_at = None;

        msg!(
            "Rebalance decision created for position: {}, requires approval: {}",
            position.key(),
            requires_human_approval
        );
        Ok(())
    }

    /// Execute a rebalancing decision
    pub fn execute_rebalance(
        ctx: Context<ExecuteRebalance>,
        _position_index: u8,
        _decision_index: u32,
        slippage_tolerance_bps: u16,
    ) -> Result<()> {
        let decision = &mut ctx.accounts.decision;
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Validate decision status
        require!(
            decision.execution_status == ExecutionStatus::Pending,
            XLiquidityEngineError::InvalidExecutionStatus
        );

        // Check if human approval is required
        if decision.requires_human_approval {
            require!(
                decision.human_approver.is_some(),
                XLiquidityEngineError::HumanApprovalRequired
            );
            if let Some(approver) = &ctx.accounts.approver {
                require!(
                    decision.human_approver.unwrap() == approver.key(),
                    XLiquidityEngineError::InvalidApprover
                );
            } else {
                return Err(XLiquidityEngineError::HumanApprovalRequired.into());
            }
        }

        // Validate slippage tolerance
        let config = &ctx.accounts.config;
        require!(
            slippage_tolerance_bps <= config.default_slippage_tolerance_bps * 2,
            XLiquidityEngineError::SlippageTooHigh
        );

        // Update position with new range
        position.current_tick_lower = decision.new_tick_lower;
        position.current_tick_upper = decision.new_tick_upper;
        position.current_price_lower = decision.new_price_lower;
        position.current_price_upper = decision.new_price_upper;
        position.last_rebalance_slot = clock.slot;
        position.last_rebalance_timestamp = clock.unix_timestamp;
        position.rebalance_count = position.rebalance_count.checked_add(1).unwrap();
        position.updated_at = clock.unix_timestamp;

        // Update decision status
        decision.execution_status = ExecutionStatus::Executed;
        decision.executed_at = Some(clock.unix_timestamp);
        // Note: execution_tx_signature and execution_slippage would be set by off-chain service

        // Create audit log
        let event_data = format!(
            "Rebalanced position {}: ticks [{}, {}], prices [{}, {}]",
            position.key(),
            decision.new_tick_lower,
            decision.new_tick_upper,
            decision.new_price_lower,
            decision.new_price_upper
        );
        create_audit_log_internal(
            &ctx.accounts.audit_log,
            AuditEventType::Rebalanced,
            Some(position.key()),
            position.owner,
            event_data.as_bytes(),
            clock,
        )?;

        msg!("Rebalance executed for position: {}", position.key());
        Ok(())
    }

    /// Verify x402 payment and grant API access
    pub fn verify_x402_payment(
        ctx: Context<VerifyX402Payment>,
        payment_id: [u8; 32],
        amount: u64,
        currency: PaymentCurrency,
        api_endpoint: String,
        api_version: String,
    ) -> Result<()> {
        let payment = &mut ctx.accounts.payment;
        let config = &ctx.accounts.config;
        let clock = Clock::get()?;

        // Validate minimum payment
        require!(
            amount >= config.x402_min_payment,
            XLiquidityEngineError::PaymentTooSmall
        );

        // Validate facilitator
        require!(
            ctx.accounts.facilitator.key() == config.x402_facilitator.unwrap_or(Pubkey::default()),
            XLiquidityEngineError::InvalidFacilitator
        );

        payment.payment_id = payment_id;
        payment.payment_bump = ctx.bumps.payment;
        payment.payer = ctx.accounts.payer.key();
        payment.payer_wallet = ctx.accounts.payer_wallet.key();
        payment.amount = amount;
        payment.currency = currency;
        payment.payment_status = PaymentStatus::Verified;
        payment.facilitator = ctx.accounts.facilitator.key();
        payment.facilitator_signature = None; // Would be set by facilitator verification
        payment.payment_tx_signature = None; // Would be set after on-chain settlement
        payment.api_endpoint = api_endpoint;
        payment.api_version = api_version;
        payment.access_granted = true;
        payment.access_expires_at = Some(clock.unix_timestamp + 3600); // 1 hour access
        payment.requested_at = clock.unix_timestamp;
        payment.verified_at = Some(clock.unix_timestamp);
        payment.settled_at = None; // Set after settlement

        // Create audit log
        create_audit_log_internal(
            &ctx.accounts.audit_log,
            AuditEventType::PaymentReceived,
            None,
            ctx.accounts.payer.key(),
            &[],
            clock,
        )?;

        msg!("x402 payment verified: {} for endpoint: {}", amount, payment.api_endpoint);
        Ok(())
    }

    /// Collect fees from a liquidity position
    pub fn collect_fees(
        ctx: Context<CollectFees>,
        _position_index: u8,
    ) -> Result<()> {
        let position = &mut ctx.accounts.position;
        let clock = Clock::get()?;

        // Validate position is active
        require!(
            position.status == PositionStatus::Active,
            XLiquidityEngineError::PositionNotActive
        );

        // Check if there are fees to collect
        require!(
            position.total_fees_earned_a > 0 || position.total_fees_earned_b > 0,
            XLiquidityEngineError::NoFeesToCollect
        );

        // Calculate protocol fees
        let config = &ctx.accounts.config;
        let _protocol_fee_a = (position.total_fees_earned_a as u128)
            .checked_mul(config.protocol_fee_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;
        let _protocol_fee_b = (position.total_fees_earned_b as u128)
            .checked_mul(config.protocol_fee_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;

        // Note: Actual token transfer would happen via CPI to token program
        // This is just updating the accounting
        // Protocol fees would be sent to config.fee_recipient in production

        // Reset fee counters (fees would be transferred off-chain)
        let fees_collected_a = position.total_fees_earned_a;
        let fees_collected_b = position.total_fees_earned_b;
        position.total_fees_earned_a = 0;
        position.total_fees_earned_b = 0;
        position.updated_at = clock.unix_timestamp;

        // Create audit log
        let event_data = format!(
            "Fees collected: {} token A, {} token B",
            fees_collected_a, fees_collected_b
        );
        create_audit_log_internal(
            &ctx.accounts.audit_log,
            AuditEventType::FeesCollected,
            Some(position.key()),
            position.owner,
            event_data.as_bytes(),
            clock,
        )?;

        msg!(
            "Fees collected from position {}: {} token A, {} token B",
            position.key(),
            fees_collected_a,
            fees_collected_b
        );
        Ok(())
    }

    /// Approve a rebalancing decision (human oversight)
    pub fn approve_rebalance(
        ctx: Context<ApproveRebalance>,
        _decision_index: u32,
    ) -> Result<()> {
        let decision = &mut ctx.accounts.decision;
        let clock = Clock::get()?;

        require!(
            decision.requires_human_approval,
            XLiquidityEngineError::ApprovalNotRequired
        );
        require!(
            decision.execution_status == ExecutionStatus::Pending,
            XLiquidityEngineError::InvalidExecutionStatus
        );

        decision.human_approver = Some(ctx.accounts.approver.key());
        decision.approval_timestamp = Some(clock.unix_timestamp);

        // Create audit log
        create_audit_log_internal(
            &ctx.accounts.audit_log,
            AuditEventType::HumanApprovalGranted,
            Some(decision.position),
            ctx.accounts.approver.key(),
            &[],
            clock,
        )?;

        msg!("Rebalance decision approved by: {}", ctx.accounts.approver.key());
        Ok(())
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Assess risk level based on prediction metrics
fn assess_risk(
    confidence: u16,
    sentiment: i16,
    volatility: u16,
) -> RiskLevel {
    // Simple risk assessment logic
    // In production, this would be more sophisticated
    if confidence < 5000 || volatility > 8000 {
        RiskLevel::Critical
    } else if confidence < 7000 || volatility > 6000 || sentiment < -5000 {
        RiskLevel::High
    } else if confidence < 8500 || volatility > 4000 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

/// Create an audit log entry (internal helper)
fn create_audit_log_internal(
    _audit_log_account: &AccountInfo,
    event_type: AuditEventType,
    position: Option<Pubkey>,
    user: Pubkey,
    _event_data: &[u8],
    _clock: Clock,
) -> Result<()> {
    // Note: This is a simplified version
    // In a real implementation, you would initialize the audit log account here
    // For now, we'll just log the event
    msg!(
        "Audit log: {:?} for user: {}, position: {:?}",
        event_type,
        user,
        position
    );
    Ok(())
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
pub enum PositionStatus {
    Active,
    Paused,
    Closed,
    Liquidated,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
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
// ERROR TYPES
// ============================================================================

#[error_code]
pub enum XLiquidityEngineError {
    #[msg("Invalid price range")]
    InvalidPriceRange,
    #[msg("Position exceeds maximum size")]
    ExceedsMaxPositionSize,
    #[msg("Trade exceeds maximum size")]
    ExceedsMaxTradeSize,
    #[msg("Position is not active")]
    PositionNotActive,
    #[msg("Rebalance too frequent")]
    RebalanceTooFrequent,
    #[msg("Invalid execution status")]
    InvalidExecutionStatus,
    #[msg("Human approval required")]
    HumanApprovalRequired,
    #[msg("Invalid approver")]
    InvalidApprover,
    #[msg("Slippage tolerance too high")]
    SlippageTooHigh,
    #[msg("Payment amount too small")]
    PaymentTooSmall,
    #[msg("Invalid facilitator")]
    InvalidFacilitator,
    #[msg("No fees to collect")]
    NoFeesToCollect,
    #[msg("Approval not required")]
    ApprovalNotRequired,
}

// ============================================================================
// INSTRUCTION CONTEXTS
// ============================================================================

#[derive(Accounts)]
pub struct InitializeProtocolConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ProtocolConfig::LEN,
        seeds = [b"protocol_config"],
        bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Fee recipient address
    pub fee_recipient: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_index: u8)]
pub struct CreateLiquidityPosition<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + LiquidityPosition::LEN,
        seeds = [b"liquidity_position", owner.key().as_ref(), &[position_index]],
        bump
    )]
    pub position: Account<'info, LiquidityPosition>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = config.config_bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: Token A vault
    pub token_a_vault: AccountInfo<'info>,
    
    /// CHECK: Token B vault
    pub token_b_vault: AccountInfo<'info>,
    
    /// CHECK: DEX pool address
    pub pool: AccountInfo<'info>,
    
    /// CHECK: Audit log account (simplified for now)
    pub audit_log: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_index: u8, decision_index: u32)]
pub struct CreateRebalanceDecision<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + RebalanceDecision::LEN,
        seeds = [b"rebalance_decision", position.key().as_ref(), &decision_index.to_le_bytes()],
        bump
    )]
    pub decision: Account<'info, RebalanceDecision>,
    
    #[account(
        mut,
        seeds = [b"liquidity_position", position.owner.as_ref(), &[position_index]],
        bump = position.position_bump
    )]
    pub position: Account<'info, LiquidityPosition>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = config.config_bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_index: u8, decision_index: u32)]
pub struct ExecuteRebalance<'info> {
    #[account(
        mut,
        seeds = [b"rebalance_decision", position.key().as_ref(), &decision_index.to_le_bytes()],
        bump = decision.decision_bump
    )]
    pub decision: Account<'info, RebalanceDecision>,
    
    #[account(
        mut,
        seeds = [b"liquidity_position", position.owner.as_ref(), &[position_index]],
        bump = position.position_bump
    )]
    pub position: Account<'info, LiquidityPosition>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = config.config_bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    /// CHECK: Approver (optional, only needed if human approval required)
    pub approver: Option<Signer<'info>>,
    
    /// CHECK: Audit log account
    pub audit_log: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(payment_id: [u8; 32])]
pub struct VerifyX402Payment<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + X402Payment::LEN,
        seeds = [b"x402_payment", payment_id.as_ref()],
        bump
    )]
    pub payment: Account<'info, X402Payment>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = config.config_bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// CHECK: Payer wallet
    pub payer_wallet: AccountInfo<'info>,
    
    /// CHECK: x402 Facilitator
    pub facilitator: AccountInfo<'info>,
    
    /// CHECK: Audit log account
    pub audit_log: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(position_index: u8)]
pub struct CollectFees<'info> {
    #[account(
        mut,
        seeds = [b"liquidity_position", position.owner.as_ref(), &[position_index]],
        bump = position.position_bump,
        constraint = position.owner == owner.key() @ XLiquidityEngineError::PositionNotActive
    )]
    pub position: Account<'info, LiquidityPosition>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = config.config_bump
    )]
    pub config: Account<'info, ProtocolConfig>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: Audit log account
    pub audit_log: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(decision_index: u32)]
pub struct ApproveRebalance<'info> {
    #[account(
        mut,
        seeds = [b"rebalance_decision", position.key().as_ref(), &decision_index.to_le_bytes()],
        bump = decision.decision_bump
    )]
    pub decision: Account<'info, RebalanceDecision>,
    
    /// CHECK: Position account (for validation)
    pub position: Account<'info, LiquidityPosition>,
    
    #[account(mut)]
    pub approver: Signer<'info>,
    
    /// CHECK: Audit log account
    pub audit_log: AccountInfo<'info>,
}

// ============================================================================
// ACCOUNT SIZE CONSTANTS
// ============================================================================

impl ProtocolConfig {
    pub const LEN: usize = 32 + // authority
        1 + // config_bump
        2 + // performance_fee_bps
        2 + // protocol_fee_bps
        32 + // fee_recipient
        1 + 32 + // x402_facilitator (Option<Pubkey>)
        8 + // x402_min_payment
        4 + 50 + // x402_api_base_url (String, max 50 chars)
        4 + // min_rebalance_interval
        4 + // max_rebalance_frequency
        2 + // default_slippage_tolerance_bps
        8 + // max_position_size
        8 + // max_single_trade_size
        8 + // require_human_approval_threshold
        4 + 20 + // default_ai_model_version (String, max 20 chars)
        4 + (32 * 10) + // ai_model_registry (Vec<Pubkey>, max 10)
        1 + // audit_log_enabled
        1 + // compliance_mode
        8 + // created_at
        8; // updated_at
}

impl LiquidityPosition {
    pub const LEN: usize = 32 + // owner
        1 + // position_bump
        32 + // token_a
        32 + // token_b
        32 + // token_a_vault
        32 + // token_b_vault
        1 + // dex
        32 + // pool_address
        1 + 32 + // position_nft (Option<Pubkey>)
        4 + // current_tick_lower
        4 + // current_tick_upper
        16 + // current_price_lower
        16 + // current_price_upper
        16 + // liquidity_amount
        8 + // total_fees_earned_a
        8 + // total_fees_earned_b
        8 + // total_value_locked
        8 + // last_rebalance_slot
        8 + // last_rebalance_timestamp
        4 + // rebalance_count
        2 + // total_return_percentage
        2 + // apy_estimate
        1 + // status
        1 + // auto_rebalance_enabled
        4 + // min_rebalance_interval
        8 + // max_position_size
        8 + // max_single_trade
        4 + (32 * 5) + // allowed_dex_programs (Vec<Pubkey>, max 5)
        8 + // created_at
        8; // updated_at
}

impl RebalanceDecision {
    pub const LEN: usize = 32 + // position
        1 + // decision_bump
        4 + // new_tick_lower
        4 + // new_tick_upper
        16 + // new_price_lower
        16 + // new_price_upper
        4 + 50 + // ai_model_version (String, max 50 chars)
        32 + // ai_model_hash
        2 + // prediction_confidence
        2 + // market_sentiment_score
        2 + // volatility_metric
        2 + // whale_activity_score
        4 + (8 * 10) + // on_chain_indicators (Vec<u64>, max 10)
        4 + 200 + // decision_reason (String, max 200 chars)
        1 + // risk_assessment
        1 + // execution_status
        1 + 100 + // execution_tx_signature (Option<String>, max 100 chars)
        1 + 2 + // execution_slippage (Option<u16>)
        1 + // requires_human_approval
        1 + 32 + // human_approver (Option<Pubkey>)
        1 + 8 + // approval_timestamp (Option<i64>)
        8 + // created_at
        1 + 8; // executed_at (Option<i64>)
}

impl X402Payment {
    pub const LEN: usize = 32 + // payment_id
        1 + // payment_bump
        32 + // payer
        32 + // payer_wallet
        8 + // amount
        1 + // currency
        1 + // payment_status
        32 + // facilitator
        1 + 64 + // facilitator_signature (Option<[u8; 64]>)
        1 + 100 + // payment_tx_signature (Option<String>, max 100 chars)
        4 + 100 + // api_endpoint (String, max 100 chars)
        4 + 20 + // api_version (String, max 20 chars)
        1 + // access_granted
        1 + 8 + // access_expires_at (Option<i64>)
        8 + // requested_at
        1 + 8 + // verified_at (Option<i64>)
        1 + 8; // settled_at (Option<i64>)
}

impl UserStrategy {
    pub const LEN: usize = 32 + // user
        1 + // strategy_bump
        4 + 50 + // strategy_name (String, max 50 chars)
        1 + // strategy_type
        1 + // risk_tolerance
        1 + // auto_rebalance_enabled
        1 + // rebalance_frequency
        2 + // price_range_width
        1 + 8 + // max_position_size (Option<u64>)
        1 + 8 + // max_single_trade (Option<u64>)
        1 + 2 + // max_slippage_bps (Option<u16>)
        4 + (32 * 10) + // preferred_tokens (Vec<Pubkey>, max 10)
        4 + (32 * 10) + // blacklisted_tokens (Vec<Pubkey>, max 10)
        1 + 20 + // preferred_ai_model (Option<String>, max 20 chars)
        1 + // require_human_approval
        8 + // created_at
        8; // updated_at
}

impl AuditLog {
    pub const LEN: usize = 32 + // log_id
        1 + // log_bump
        1 + // event_type
        1 + 32 + // position (Option<Pubkey>)
        32 + // user
        4 + 500 + // event_data (Vec<u8>, max 500 bytes)
        32 + // event_hash
        1 + 50 + // ai_model_version (Option<String>, max 50 chars)
        1 + 200 + // decision_rationale (Option<String>, max 200 chars)
        1 + 4 + (2 * 20) + // prediction_scores (Option<Vec<u16>>, max 20)
        1 + 10 + // regulatory_jurisdiction (Option<String>, max 10 chars)
        8 + // created_at
        8; // slot
}
