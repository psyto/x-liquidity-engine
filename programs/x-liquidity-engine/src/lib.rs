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

#[derive(Accounts)]
pub struct Initialize {}
