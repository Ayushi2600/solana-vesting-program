use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{Mint, MintTo, Token, TokenAccount, Transfer};
// use anchor_lang::solana_program::program::invoke;
// use anchor_lang::solana_program::system_instruction;
use std::mem::size_of;

// programID
declare_id!("3XDaifPQuLu23EQ5SnQkkVssxDj4TfCApxtyxsKSA6GU");

// Constant size
pub const ANCHOR_DISCRIMINATOR_SIZE: usize = 8;

// Vesting Types
const VESTING_TYPE_IMMEDIATE: u8 = 0;
const VESTING_TYPE_LINEAR: u8 = 1;

// Lamports per SOL and Token Decimals
const LAMPORTS_PER_SOL: u64 = 1000000000;
const TOKEN_DECIMALS: u64 = 1000000000;

#[program]
pub mod vesting_program {
    use super::*;

    // ========= Initialize Function ===============
    pub fn initialize(
        ctx: Context<InitializeIco>,
        ico_start_time: u64,
        ico_end_time: u64,
        tge_time: u64,
        min_buy_amount: u64,
        max_buy_amount: u64,
        tokens_per_sol: u64,  // tokens per SOL
        tokens_per_usdc: u64, // tokens per USDC
        tokens_per_usdt: u64, // tokens per USDT
        seconds_per_day: u64,
    ) -> Result<()> {
        let config_account = &mut ctx.accounts.ico_config;

        config_account.authority = ctx.accounts.authority.key();

        config_account.sol_treasury = ctx.accounts.sol_treasury.key();
        config_account.usdc_treasury = ctx.accounts.usdc_treasury.key();
        config_account.usdt_treasury = ctx.accounts.usdt_treasury.key();
        config_account.prize_treasury = ctx.accounts.reward_token_treasury.key();

        config_account.usdc_mint = ctx.accounts.usdc_mint.key();
        config_account.usdt_mint = ctx.accounts.usdt_mint.key();
        config_account.reward_token_mint = ctx.accounts.reward_token_mint.key();

        config_account.usdc_decimals = ctx.accounts.usdc_mint.decimals;
        config_account.usdt_decimals = ctx.accounts.usdt_mint.decimals;
        config_account.reward_token_decimals = ctx.accounts.reward_token_mint.decimals;

        config_account.ico_start_time = ico_start_time;
        config_account.ico_end_time = ico_end_time;
        config_account.tge_time = tge_time;

        config_account.min_amount = min_buy_amount;
        config_account.max_amount = max_buy_amount;

        config_account.tokens_per_sol = tokens_per_sol;
        config_account.tokens_per_usdc = tokens_per_usdc;
        config_account.tokens_per_usdt = tokens_per_usdt;

        config_account.seconds_per_day = seconds_per_day;

        config_account.paused = false;
        config_account.total_allocated = 0;
        config_account.total_user_allocated = 0;
        config_account.total_prize_deposited = 0;

        config_account.total_claimed = 0;

        msg!("=========ICO Config Initialized===========");
        msg!("Authority: {}", ctx.accounts.authority.key());
        msg!("Reward Token Mint: {}", config_account.reward_token_mint);
        msg!("SOL Treasury: {}", config_account.sol_treasury);
        msg!("USDC Treasury: {}", config_account.usdc_treasury);
        msg!("USDT Treasury: {}", config_account.usdt_treasury);
        msg!("USDC Mint: {}", config_account.usdc_mint);
        msg!("USDT Mint: {}", config_account.usdt_mint);
        msg!("USDC Decimals: {}", ctx.accounts.usdc_mint.decimals);
        msg!("USDT Decimals: {}", ctx.accounts.usdt_mint.decimals);
        msg!("Reward Token Decimals: {}", ctx.accounts.reward_token_mint.decimals);
        msg!("ICO Start Time: {}", config_account.ico_start_time);
        msg!("ICO End Time: {}", config_account.ico_end_time);
        msg!("TGE Time: {}", config_account.tge_time);
        msg!("Set Time for vesting: {}", config_account.seconds_per_day);
        msg!("Min Purchase Amount: {}", config_account.min_amount);
        msg!("Max Purchase Amount: {}", config_account.max_amount);
        msg!(
            "Rate (SOL): {} tokens per SOL",
            config_account.tokens_per_sol
        );
        msg!(
            "Rate (USDC): {} tokens per USDC",
            config_account.tokens_per_usdc
        );
        msg!(
            "Rate (USDT): {} tokens per USDT",
            config_account.tokens_per_usdt
        );

        emit!(InitializeEvent {
            authority: ctx.accounts.authority.key(),
            ico_start_time,
            ico_end_time,
            tge_time,
        });
        Ok(())
    }

    // ========== Whitelist investor or add investor (allocating token manually)=============
    pub fn whitelist_investor_by_admin(
        ctx: Context<InvestorEntry>,
        investor_address: Pubkey,
        amount: u64,
        vesting_type: u8,
    ) -> Result<()> {
        // Get the authority directly from the ico_config account
        let config_authority = ctx.accounts.ico_config.authority;

        // AdminOnly check - ensure the signer is the authority from ico_config
        require!(
            ctx.accounts.authority.key() == config_authority,
            CustomError::UnauthorizedAccess
        );

        require!(
            (vesting_type == VESTING_TYPE_IMMEDIATE || vesting_type == VESTING_TYPE_LINEAR),
            CustomError::InvalidVestingType
        );

        let config_account = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        msg!("current_time ===========> {}", current_time);

        // check the TGE start 
        require!(
            current_time <= config_account.tge_time,
            CustomError::TGEDateInvalid
        );

        // Buy time must be between ICO start & end time
        require!(
            (config_account.ico_start_time <= current_time
                && current_time <= config_account.ico_end_time),
            CustomError::ICOPhaseInvalid
        );

        let investor = &mut ctx.accounts.investor_details;

        // Calculate tokens to allocate
        let tokens_to_allocate = amount;

        // Update allocation regardless of whether this is first purchase
        let total_allocation = investor
            .allocation
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::CalculationOverflow)?;

        require!(
            tokens_to_allocate >= config_account.min_amount
                && tokens_to_allocate <= config_account.max_amount,
            CustomError::InvalidBuyAmount
        );

        investor.address = investor_address;
        investor.allocation = total_allocation;

        config_account.total_allocated = config_account
            .total_allocated
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::ArithmeticOverflow)?;

        msg!("tokens_to_allocate ====> {:?}", tokens_to_allocate);

        investor.vesting_type = vesting_type;
        investor.released_tokens = investor
            .released_tokens
            .checked_add(0)
            .ok_or(CustomError::ArithmeticOverflow)?;

        investor.last_claimed = investor
            .last_claimed
            .checked_add(0)
            .ok_or(CustomError::ArithmeticOverflow)?;
        investor.claimed_tokens = investor
            .claimed_tokens
            .checked_add(0)
            .ok_or(CustomError::ArithmeticOverflow)?;
        investor.blocked = false;
        // investor.whitelisted_at = current_time;

        if investor.whitelisted_at == 0 {
            investor.whitelisted_at = current_time;
        }

        msg!("Investor Whitelisted Successfully!");
        msg!("Investor Address: {}", investor.address);
        msg!("Allocation: {}", investor.allocation);
        msg!("Vesting Type: {:?}", investor.vesting_type);
        msg!("Released Tokens: {}", investor.released_tokens);
        msg!("Cliff End (timestamp): {}", investor.cliff_end);
        msg!("Last Claimed: {}", investor.last_claimed);
        msg!("Claimed Tokens: {}", investor.claimed_tokens);
        msg!("Whitelisted At: {}", investor.whitelisted_at);

        emit!(InvestorWhitelisted {
            investor: investor.address,
            allocation: investor.allocation,
            vesting_type: investor.vesting_type,
            cliff_end: investor.cliff_end,
        });

        Ok(())
    }

    // ============== Buy Token functionality (investor Action) ==================

    // With Sol
    pub fn buy_tokens_with_sol(
        ctx: Context<BuyTokensWithSol>,
        investor_address: Pubkey,
        amount: u64,
        vesting_type: u8,
    ) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(!config.paused, CustomError::ICOIsPaused);

        // check the TGE start 
        require!(current_time <= config.tge_time, CustomError::TGEDateInvalid);

        require!(
            current_time >= config.ico_start_time && current_time <= config.ico_end_time,
            CustomError::ICOPhaseInvalid
        );

        // Fetch balances before transaction
        let buyer_balance_before = ctx.accounts.buyer.lamports();

        msg!("Buyer's SOL pre-balance: {}", buyer_balance_before);

        // Calculate tokens to allocate
        let tokens_to_allocate = amount
            .checked_mul(config.tokens_per_sol)
            .ok_or(CustomError::CalculationOverflow)?
            .checked_div(LAMPORTS_PER_SOL)
            .ok_or(CustomError::CalculationOverflow)?;

        msg!("tokens_to_allocate ====> {:?}", tokens_to_allocate);

        require!(
            tokens_to_allocate >= config.min_amount && tokens_to_allocate <= config.max_amount,
            CustomError::InvalidBuyAmount
        );

        require!(
            (vesting_type == VESTING_TYPE_IMMEDIATE || vesting_type == VESTING_TYPE_LINEAR),
            CustomError::InvalidVestingType
        );

        // Transfer SOL from buyer to SOL treasury
        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.buyer.to_account_info(),
            to: ctx.accounts.sol_treasury.to_account_info(),
        };
        let cpi_context =
            CpiContext::new(ctx.accounts.system_program.to_account_info(), cpi_accounts);

        system_program::transfer(cpi_context, amount)?;

        msg!("Transferred {} lamports from buyer to SOL treasury", amount);

        // Get investor details
        let investor = &mut ctx.accounts.investor_details;

        // Validate vesting_type consistency
        if investor.vesting_type != 0 {
            require!(
                investor.vesting_type == vesting_type,
                CustomError::MismatchedVestingType
            );
        }
        // Apply whitelisting logic using the private helper function
        process_investor_whitelist(
            investor,
            ctx.accounts.buyer.key(),
            config.tge_time,
            vesting_type,
            tokens_to_allocate,
        )?;

        // Update allocation regardless of whether this is first purchase
        let total_allocation = investor
            .allocation
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::CalculationOverflow)?;

        investor.allocation = total_allocation;

        config.total_allocated = config
            .total_allocated
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::ArithmeticOverflow)?;

        // Fetch balances after transaction
        let buyer_balance_after = ctx.accounts.buyer.lamports();

        msg!("Buyer's SOL post-balance: {}", buyer_balance_after);
        msg!("Total allocation now: {}", investor.allocation);

        let rent = Rent::get()?;
        require!(
            ctx.accounts.buyer.lamports() >= rent.minimum_balance(0),
            CustomError::BuyerNotRentExempt
        );

        emit!(TokenPurchaseEventForSol {
            buyer: ctx.accounts.buyer.key(),
            sol_amount: amount,
            token_amount: tokens_to_allocate,
            timestamp: current_time,
        });
        Ok(())
    }

    // Update the buy_token_with_usdc function to include vesting_type parameter
    pub fn buy_token_with_usdc(
        ctx: Context<BuyTokenWithUsdc>,
        investor_address: Pubkey,
        amount: u64,
        vesting_type: u8,
    ) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(!config.paused, CustomError::ICOIsPaused);

        // check the TGE start 
        require!(current_time <= config.tge_time, CustomError::TGEDateInvalid);

        require!(
            current_time >= config.ico_start_time && current_time <= config.ico_end_time,
            CustomError::ICOPhaseInvalid
        );

        // Calculate tokens to allocate
        let tokens_to_allocate = amount
            .checked_mul(config.tokens_per_usdc)
            .ok_or(CustomError::CalculationOverflow)?
            .checked_div(TOKEN_DECIMALS)
            .ok_or(CustomError::CalculationOverflow)?;

        require!(
            tokens_to_allocate >= config.min_amount && tokens_to_allocate <= config.max_amount,
            CustomError::InvalidBuyAmount
        );

        require!(
            (vesting_type == VESTING_TYPE_IMMEDIATE || vesting_type == VESTING_TYPE_LINEAR),
            CustomError::InvalidVestingType
        );

        // Fetch balances before transaction
        let buyer_balance_before = ctx.accounts.buyer_token_account.amount;

        msg!("Buyer's USDC Pre Balance: {}", buyer_balance_before);

        // Transfer USDC from buyer token account to USDC treasury
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.usdc_treasury.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        anchor_spl::token::transfer(cpi_context, amount)?;

        msg!("Transferred {} USDC from buyer to USDC treasury", amount);

        // Get investor details
        let investor = &mut ctx.accounts.investor_details;

        // Validate vesting_type consistency
        if investor.vesting_type != 0 {
            require!(
                investor.vesting_type == vesting_type,
                CustomError::MismatchedVestingType
            );
        }
        // Apply whitelisting logic using the private helper function
        process_investor_whitelist(
            investor,
            ctx.accounts.buyer.key(),
            config.tge_time,
            vesting_type,
            tokens_to_allocate,
        )?;

        // Update allocation regardless of whether this is first purchase
        let total_allocation = investor
            .allocation
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::CalculationOverflow)?;

        investor.allocation = total_allocation;

        config.total_allocated = config
            .total_allocated
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::ArithmeticOverflow)?;

        // We need to reload the account to get the updated balance
        let buyer_token_account = &mut ctx.accounts.buyer_token_account;
        buyer_token_account.reload()?;

        let remaining_usdc_bal = buyer_token_account.amount;

        msg!("Buyer USDC Post Balance: {}", remaining_usdc_bal);
        msg!("Total allocation now: {}", investor.allocation);

        emit!(TokenPurchaseEventForUsdc {
            buyer: ctx.accounts.buyer.key(),
            usdc_amount: amount,
            token_amount: tokens_to_allocate,
            timestamp: current_time,
            usdc_treasury: ctx.accounts.usdc_treasury.key(),
        });

        Ok(())
    }

    // Update the buy_token_with_usdt function to include vesting_type parameter
    pub fn buy_token_with_usdt(
        ctx: Context<BuyTokenWithUsdt>,
        investor_address: Pubkey,
        amount: u64,
        vesting_type: u8,
    ) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(!config.paused, CustomError::ICOIsPaused);

        // check the TGE start  
        require!(current_time <= config.tge_time, CustomError::TGEDateInvalid);

        require!(
            current_time >= config.ico_start_time && current_time <= config.ico_end_time,
            CustomError::ICOPhaseInvalid
        );

        require!(
            (vesting_type == VESTING_TYPE_IMMEDIATE || vesting_type == VESTING_TYPE_LINEAR),
            CustomError::InvalidVestingType
        );

        // Calculate tokens to allocate
        let tokens_to_allocate = amount
            .checked_mul(config.tokens_per_usdt)
            .ok_or(CustomError::CalculationOverflow)?
            .checked_div(TOKEN_DECIMALS)
            .ok_or(CustomError::CalculationOverflow)?;

        require!(
            tokens_to_allocate >= config.min_amount && tokens_to_allocate <= config.max_amount,
            CustomError::InvalidBuyAmount
        );

        let buyer_balance_before = ctx.accounts.buyer_token_account.amount;

        msg!("Buyer's USDT Pre Balance: {}", buyer_balance_before);

        // Transfer USDT from buyer token account to USDT treasury
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.usdt_treasury.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        anchor_spl::token::transfer(cpi_context, amount)?;

        msg!("Transferred {} USDT from buyer to USDT treasury", amount);

        // Get investor details
        let investor = &mut ctx.accounts.investor_details;

        // Validate vesting_type consistency
        if investor.vesting_type != 0 {
            require!(
                investor.vesting_type == vesting_type,
                CustomError::MismatchedVestingType
            );
        }
        // Apply whitelisting logic using the private helper function
        process_investor_whitelist(
            investor,
            ctx.accounts.buyer.key(),
            config.tge_time,
            vesting_type,
            tokens_to_allocate,
        )?;

        // Update allocation regardless of whether this is first purchase
        let total_allocation = investor
            .allocation
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::CalculationOverflow)?;

        investor.allocation = total_allocation;

        config.total_user_allocated = config
            .total_user_allocated
            .checked_add(tokens_to_allocate)
            .ok_or(CustomError::ArithmeticOverflow)?;

        msg!("Total tokens sold so far: {}", config.total_user_allocated);

        // We need to reload the account to get the updated balance
        let buyer_token_account = &mut ctx.accounts.buyer_token_account;
        buyer_token_account.reload()?;

        let remaining_usdt_bal = buyer_token_account.amount;

        msg!("Buyer's USDT Post Balance: {}", remaining_usdt_bal);
        msg!("Total allocation now: {}", investor.allocation);

        emit!(TokenPurchaseEventForUsdt {
            buyer: ctx.accounts.buyer.key(),
            usdt_amount: amount,
            token_amount: tokens_to_allocate,
            timestamp: current_time,
            usdt_treasury: ctx.accounts.usdt_treasury.key(),
        });

        Ok(())
    }


    pub fn set_ico_dates(
        ctx: Context<SetIcoConfig>,
        ico_start_time: u64,
        ico_end_time: u64,
    ) -> Result<()> {
        // Check that the caller is the authority
        require!(
            ctx.accounts.authority.key() == ctx.accounts.ico_config.authority,
            CustomError::UnauthorizedAccess
        );

        // Update the ICO time
        ctx.accounts.ico_config.ico_start_time = ico_start_time;
        ctx.accounts.ico_config.ico_end_time = ico_end_time;

        msg!("ICO Start time updated to {}", ico_start_time);
        msg!("ICO End time updated to {}", ico_end_time);

        emit!(ICODateChanged {
            authority: ctx.accounts.authority.key(),
            ico_start_time: ico_start_time,
            ico_end_time: ico_end_time,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });
        Ok(())
    }

    pub fn remove_investor(ctx: Context<RemoveInvestor>, investor_address: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        // Ensure the signer is the authority from the config
        require!(
            ctx.accounts.authority.key() == config.authority,
            CustomError::UnauthorizedAccess
        );

        msg!(
            "Investor {} removed successfully by admin {}",
            investor_address,
            ctx.accounts.authority.key()
        );

        let investor = &mut ctx.accounts.investor_details;

        let remaining_allocation = investor
            .allocation
            .checked_sub(investor.released_tokens)
            .ok_or(CustomError::ArithmeticOverflow)?;
        msg!(
            "remaining_allocation which is going to be reversed ========> {:?}",
            remaining_allocation
        );

        config.total_allocated = config
            .total_allocated
            .checked_sub(remaining_allocation)
            .ok_or(CustomError::ArithmeticOverflow)?;


        **investor = Investor {
            address: Pubkey::default(),
            allocation: 0,
            released_tokens: 0,
            claimed_tokens: 0,
            blocked: false,
            vesting_type: 0,
            cliff_end: 0,
            last_claimed: 0,
            whitelisted_at: 0
        };

        // Emit an event for the removal
        emit!(InvestorRemoved {
            investor: investor_address,
            removed_by: ctx.accounts.authority.key(),
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }

    pub fn reset_sol_treasure(ctx: Context<UpdateTreasuryWallet>) -> Result<()> {
        // Update the sol_treasury 
        ctx.accounts.ico_config.sol_treasury = ctx.accounts.new_wallet.key();
        msg!(
            "SOL Treasury has been updated to {}",
            ctx.accounts.new_wallet.key()
        );

        emit!(SolTreasuryUpdated {
            authority: ctx.accounts.authority.key(),
            sol_treasury: ctx.accounts.new_wallet.key(),
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }

    // Reset mint address token_type =0 (prize), token_type = 1 (usdc), token_type = 2 (usdt)
    pub fn set_mint_address(
        ctx: Context<SetMintAddress>,
        token_type: u8,
        token: Pubkey,
    ) -> Result<()> {
        let mint = &ctx.accounts.mint_account;
        // Check that the caller is the authority
        require!(
            ctx.accounts.authority.key() == ctx.accounts.ico_config.authority,
            CustomError::UnauthorizedAccess
        );

        // Check oken_type =0 (prize), token_type = 1 (usdc), token_type = 2 (usdt)
        require!(
            token_type >= 0 && token_type <= 2,
            CustomError::InvalidTokenType
        );

        if token_type == 0 {
            // Update the $Prize Token
            ctx.accounts.ico_config.reward_token_mint = token;
            ctx.accounts.ico_config.reward_token_decimals = mint.decimals;
            msg!("$prize Token updated to {}", token);

            emit!(PrizeTokenUpdated {
                authority: ctx.accounts.authority.key(),
                token: token,
                timestamp: Clock::get()?.unix_timestamp as u64,
            });
        } else if token_type == 1 {
            // Update the usdc_mint
            ctx.accounts.ico_config.usdc_mint = token;
            ctx.accounts.ico_config.usdc_decimals = mint.decimals;
            msg!("Usdc Mint Address updated to {}", token);

            emit!(UsdcAddressUpdated {
                authority: ctx.accounts.authority.key(),
                token: token,
                timestamp: Clock::get()?.unix_timestamp as u64,
            });
        } else if token_type == 2 {
            // Update the usdt_mint
            ctx.accounts.ico_config.usdt_mint = token;
            ctx.accounts.ico_config.usdt_decimals = mint.decimals;
            msg!("Usdt Mint Address updated to {}", token);

            emit!(UsdtAddressUpdated {
                authority: ctx.accounts.authority.key(),
                token: token,
                timestamp: Clock::get()?.unix_timestamp as u64,
            });
        }

        Ok(())
    }

    pub fn set_tge_date(ctx: Context<SetIcoConfig>, new_tge_time: u64) -> Result<()> {
        // Check that the caller is the authority
        require!(
            ctx.accounts.authority.key() == ctx.accounts.ico_config.authority,
            CustomError::UnauthorizedAccess
        );

        // Update the TGE time
        ctx.accounts.ico_config.tge_time = new_tge_time;

        msg!("TGE time updated to {}", new_tge_time);

        emit!(TGEDateChanged {
            authority: ctx.accounts.authority.key(),
            new_tge_time: new_tge_time,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        require!(ctx.accounts.authority.key() == config.authority, CustomError::UnauthorizedAccess);
        config.paused = paused;

        emit!(PausedEvent {
            authority: ctx.accounts.authority.key(),
            paused,
        });

        Ok(())
    }


    pub fn reset_seconds_per_day(ctx: Context<SetIcoConfig>, nos_of_seconds: u64) -> Result<()> {
        // Check that the caller is the authority
        require!(
            ctx.accounts.authority.key() == ctx.accounts.ico_config.authority,
            CustomError::UnauthorizedAccess
        );

        // Update the TGE time
        let prev = ctx.accounts.ico_config.seconds_per_day;
        ctx.accounts.ico_config.seconds_per_day = nos_of_seconds;

        msg!("Seconds per day has updated to {}", nos_of_seconds);

        emit!(SecondsPerDayChanged {
            authority: ctx.accounts.authority.key(),
            new_value: nos_of_seconds,
            old_value: prev,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }

    pub fn withdraw_sol(
        ctx: Context<WithdrawSol>,
        amount: u64,
        recipient_address: Pubkey,
    ) -> Result<()> {
        // Check sufficient funds
        require!(
            ctx.accounts.sol_treasury.lamports() >= amount,
            CustomError::InsufficientFunds
        );

        // Transfer funds

        // Transfer SOL from SOL treasury to recipient

        let cpi_accounts = system_program::Transfer {
            from: ctx.accounts.sol_treasury.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.system_program.to_account_info(), cpi_accounts);

        system_program::transfer(cpi_context, amount)?;

        // Emit event
        let clock = Clock::get()?;

        emit!(WithdrawnSol {
            authority: ctx.accounts.sol_treasury.key(),
            recipient: recipient_address,
            amount,
            timestamp: clock.unix_timestamp as u64,
        });

        msg!(
            "Withdrawn {} SOL from treasury {} to {}",
            amount,
            ctx.accounts.sol_treasury.key(),
            recipient_address
        );

        Ok(())
    }

    pub fn withdraw_prize_tokens(ctx: Context<PrizeTokensTransfer>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(
            ctx.accounts.authority.key() == ctx.accounts.ico_config.authority,
            CustomError::UnauthorizedAccess
        );

        require!(
            ctx.accounts.reward_token_treasury.amount >= amount,
            CustomError::InsufficientFunds
        );

        let vault_auth = &ctx.bumps.vault_authority;
        let seeds = &[b"vault_authority".as_ref(), &[ctx.bumps.vault_authority]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_token_treasury.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer
        );

        anchor_spl::token::transfer(cpi_ctx, amount)?;

        msg!("Withdrawn {} reward tokens from treasury", amount);

        emit!(PrizeWithdrawTokens {
            authority: ctx.accounts.vault_authority.key(),
            amount,
            timestamp: current_time,
        });

        Ok(())
    }


    pub fn withdraw_usdc_tokens(ctx: Context<UsdcTokensTransfer>, amount: u64) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(
            ctx.accounts.usdc_treasury.amount >= amount,
            CustomError::InsufficientFunds
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.usdc_treasury.to_account_info(),
            to: ctx.accounts.recipient_usdc_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        anchor_spl::token::transfer(cpi_context, amount)?;

        msg!("Withdrawn {} reward tokens from usdc treasury", amount);

        emit!(UsdcWithdrawTokens {
            authority: ctx.accounts.usdc_treasury.key(),
            amount: amount,
            timestamp: current_time,
        });

        Ok(())
    }

    pub fn withdraw_usdt_tokens(ctx: Context<UsdtTokensTransfer>, amount: u64) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(
            ctx.accounts.usdt_treasury.amount >= amount,
            CustomError::InsufficientFunds
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.usdt_treasury.to_account_info(),
            to: ctx.accounts.recipient_usdt_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        anchor_spl::token::transfer(cpi_context, amount)?;

        msg!("Withdrawn {} reward tokens from usdt treasury", amount);

        emit!(UsdtWithdrawTokens {
            authority: ctx.accounts.usdt_treasury.key(),
            amount: amount,
            timestamp: current_time,
        });

        Ok(())
    }

    pub fn block_investor(ctx: Context<BlockInvestor>, investor_address: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;
        let investor = &mut ctx.accounts.investor_details;

        require!(
            ctx.accounts.authority.key() == config.authority,
            CustomError::UnauthorizedAccess
        );

        investor.blocked = true;

        msg!("Investor {} has been blocked", investor.address);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        emit!(InvestorBlocked {
            investor: investor.address,
            blocked_by: ctx.accounts.authority.key(),
            timestamp: current_time,
        });
        Ok(())
    }

    pub fn deposit_prize(ctx: Context<DepositPrize>, amount: u64) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        require!(
            ctx.accounts.authority.key() == config.authority,
            CustomError::UnauthorizedAccess
        );

        // Transfer tokens from authority token account to reward token treasury
        let cpi_accounts = Transfer {
            from: ctx.accounts.authority_token_account.to_account_info(),
            to: ctx.accounts.reward_token_treasury.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

        anchor_spl::token::transfer(cpi_context, amount)?;

        // Update the config's total_allocated
        config.total_prize_deposited = config
            .total_prize_deposited
            .checked_add(amount)
            .ok_or(CustomError::ArithmeticOverflow)?;

        msg!("Deposited {} to treasury", amount);
        msg!("Total prize tokens now: {}", config.total_prize_deposited);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        emit!(PrizeDeposited {
            depositor: ctx.accounts.authority.key(),
            amount: amount,
            timestamp: current_time,
        });

        Ok(())
    }

    // Transfer Ownership =======================
    pub fn transfer_ownership(
        ctx: Context<TransferOwnership>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;
        require!(
            ctx.accounts.authority.key() == config.authority,
            CustomError::UnauthorizedAccess
        );
        require!(
            new_authority != Pubkey::default(),
            CustomError::InvalidAddress
        );
        config.authority = new_authority;

        msg!("Ownership transferred to {}", new_authority);

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        emit!(OwnershipTransferred {
            previous_owner: ctx.accounts.authority.key(),
            new_owner: new_authority,
            timestamp: current_time,
        });

        Ok(())
    }

    // Renounce Ownership =========================

    pub fn renounce_ownership(ctx: Context<RenounceOwnership>) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;
        require!(
            ctx.accounts.authority.key() == config.authority,
            CustomError::UnauthorizedAccess
        );

        config.authority = Pubkey::default();

        msg!("Ownership renounced");

        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        emit!(OwnershipRenounced {
            previous_owner: ctx.accounts.authority.key(),
            timestamp: current_time,
        });

        Ok(())
    }

    pub fn claim_tokens(ctx: Context<ClaimTokens>, investor_address: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.ico_config;

        let investor = &mut ctx.accounts.investor_details;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        // require_keys_eq!(ctx.accounts.investor.key(), investor.address, CustomError::UnauthorizedAccess);

        require!(
            investor.address == ctx.accounts.investor.key(),
            CustomError::UnauthorizedAccess
        );

        // Check if investor is blocked
        require!(!investor.blocked, CustomError::InvestorIsBlocked);

        // Check if TGE has happened
        require!(current_time >= config.tge_time, CustomError::TGEDateInvalid);

        // Calculate claimable tokens based on vesting type
        let (claimable_tokens, _released_tokens) = calculate_claimable_tokens(
            investor.vesting_type,
            investor.allocation,
            investor.claimed_tokens,
            config.tge_time,
            config.seconds_per_day,
            current_time,
        )?;

        msg!("claimable_tokens =====> {}", claimable_tokens);

        // Check if there are tokens to claim
        require!(claimable_tokens > 0, CustomError::NoTokensAvailableToClaim);

        // Check if treasury has enough tokens
        require!(
            ctx.accounts.reward_token_treasury.amount >= claimable_tokens,
            CustomError::InsufficientFunds
        );

        let vault_auth = &ctx.bumps.vault_authority;
        let seeds = &[b"vault_authority".as_ref(), &[ctx.bumps.vault_authority]];
        let signer = &[&seeds[..]];

        // Transfer tokens to investor
        let cpi_accounts = Transfer {
            from: ctx.accounts.reward_token_treasury.to_account_info(),
            to: ctx.accounts.investor_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );

        let tokens_amount = claimable_tokens
            .checked_mul(TOKEN_DECIMALS)
            .ok_or(CustomError::ArithmeticOverflow)?;

        anchor_spl::token::transfer(cpi_context, tokens_amount)?;

        // Update investor claimed tokens
        investor.claimed_tokens = investor
            .claimed_tokens
            .checked_add(claimable_tokens)
            .ok_or(CustomError::ArithmeticOverflow)?;

        // Update released tokens
        investor.released_tokens = investor
            .released_tokens
            .checked_add(claimable_tokens)
            .ok_or(CustomError::ArithmeticOverflow)?;

        // Update last claimed timestamp
        investor.last_claimed = current_time;
        investor.cliff_end = get_next_cliff_end(
            investor.vesting_type,
            investor.cliff_end,
            config.seconds_per_day,
            config.tge_time,
            current_time,
        )?;

        // Update total claimed in config
        config.total_claimed = config
            .total_claimed
            .checked_add(claimable_tokens)
            .ok_or(CustomError::ArithmeticOverflow)?;

        msg!("Tokens claimed successfully!");
        msg!("Investor: {}", investor.address);
        msg!("Claimed amount: {}", claimable_tokens);
        msg!("Total claimed so far: {}", investor.claimed_tokens);
        msg!(
            "Remaining to claim: {}",
            investor.allocation.saturating_sub(investor.claimed_tokens)
        );

        emit!(TokensClaimed {
            investor: investor.address,
            amount: claimable_tokens,
            timestamp: current_time,
            remaining: investor.allocation.saturating_sub(investor.claimed_tokens),
        });

        Ok(())
    }

    // Helper function to calculate claimable tokens based on vesting type
    pub fn determine_claimable_tokens(
        ctx: Context<Calculate>,
        vesting_type: u8,
        total_allocation: u64,
        already_claimed: u64,
        tge_time: u64,
        seconds_per_day: u64,
        current_time: u64,
    ) -> Result<(u64, u64)> {
        // Calculate the total releasable tokens based on vesting type and current time
        let total_releasable = match vesting_type {
            // Immediate vesting - 10% at TGE, rest linearly over 365 days
            VESTING_TYPE_IMMEDIATE => {
                // 10% at TGE initially

                let mut initial_release = total_allocation
                    .checked_mul(10)
                    .ok_or(CustomError::ArithmeticOverflow)?
                    .checked_div(100)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Remaining 90% for daily vesting
                let mut remaining_allocation = total_allocation
                    .checked_sub(initial_release)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Daily vesting amount
                let daily_release = remaining_allocation
                    .checked_div(365)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Days passed since TGE
                let mut days_passed = if current_time <= tge_time {
                    0
                } else {
                    current_time
                        .checked_sub(tge_time)
                        .ok_or(CustomError::ArithmeticOverflow)?
                        .checked_div(seconds_per_day)
                        .ok_or(CustomError::ArithmeticOverflow)?
                };

                msg!("days_passed =============> {:?}", days_passed);

                // Cap at 365 days
                let effective_days = std::cmp::min(days_passed, 365);
                msg!("effective_days =============> {:?}", effective_days);

                // Calculate vested tokens
                let vested_from_daily = daily_release
                    .checked_mul(effective_days)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                msg!("vested_from_daily =============> {:?}", vested_from_daily);

                // Total releasable: initial + vested from daily
                initial_release
                    .checked_add(vested_from_daily)
                    .ok_or(CustomError::ArithmeticOverflow)?
            }

            // Linear vesting with milestones
            VESTING_TYPE_LINEAR => {
                // Initial 5% at TGE
                let mut initial_release = total_allocation
                    .checked_mul(5)
                    .ok_or(CustomError::ArithmeticOverflow)?
                    .checked_div(100)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Calculate seconds in months for different milestones
                let mut month_in_seconds = seconds_per_day
                    .checked_mul(30)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Define milestones in seconds from TGE
                let mut milestone_8_months = month_in_seconds
                    .checked_mul(8)
                    .ok_or(CustomError::ArithmeticOverflow)?;
                let mut milestone_18_months = month_in_seconds
                    .checked_mul(18)
                    .ok_or(CustomError::ArithmeticOverflow)?;
                let mut milestone_24_months = month_in_seconds
                    .checked_mul(24)
                    .ok_or(CustomError::ArithmeticOverflow)?;
                let mut milestone_36_months = month_in_seconds
                    .checked_mul(36)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                // Calculate time elapsed since TGE
                let mut time_elapsed = if current_time <= tge_time {
                    0
                } else {
                    current_time
                        .checked_sub(tge_time)
                        .ok_or(CustomError::ArithmeticOverflow)?
                };

                // Calculate vested amount based on time passed
                let mut vested_amount = initial_release; // Start with TGE release (5%)

                // TGE + 8 months: 10%
                if time_elapsed >= milestone_8_months {
                    vested_amount = vested_amount
                        .checked_add(
                            total_allocation
                                .checked_mul(10)
                                .ok_or(CustomError::ArithmeticOverflow)?
                                .checked_div(100)
                                .ok_or(CustomError::ArithmeticOverflow)?,
                        ).ok_or(CustomError::ArithmeticOverflow)?;
                }

                // TGE + 18 months: 20%
                if time_elapsed >= milestone_18_months {
                    vested_amount = vested_amount
                        .checked_add(
                            total_allocation
                                .checked_mul(20)
                                .ok_or(CustomError::ArithmeticOverflow)?
                                .checked_div(100)
                                .ok_or(CustomError::ArithmeticOverflow)?,
                        ).ok_or(CustomError::ArithmeticOverflow)?;
                }

                // TGE + 24 months: 20%
                if time_elapsed >= milestone_24_months {
                    vested_amount = vested_amount
                        .checked_add(
                            total_allocation
                                .checked_mul(20)
                                .ok_or(CustomError::ArithmeticOverflow)?
                                .checked_div(100)
                                .ok_or(CustomError::ArithmeticOverflow)?,
                        ).ok_or(CustomError::ArithmeticOverflow)?;
                }

                // TGE + 36 months: 45%
                if time_elapsed >= milestone_36_months {
                    vested_amount = vested_amount
                        .checked_add(
                            total_allocation
                                .checked_mul(45)
                                .ok_or(CustomError::ArithmeticOverflow)?
                                .checked_div(100)
                                .ok_or(CustomError::ArithmeticOverflow)?,
                        ).ok_or(CustomError::ArithmeticOverflow)?;
                }

                vested_amount
            }
            _ => return Err(CustomError::InvalidVestingType.into()),
        };

        // Calculate claimable tokens (total releasable minus already claimed)
        msg!("already_claimed ===========> {}", already_claimed);
        msg!("total_releasable ===========> {}", total_releasable);
        let claimable = total_releasable.saturating_sub(already_claimed);
        msg!("claimable ===========> {}", claimable);

        Ok((claimable, total_releasable))
    }

    // =========== get functions ========================

    pub fn get_ico_dates(ctx: Context<GetIcoDates>) -> Result<()> {
        let config = &ctx.accounts.ico_config;

        msg!("ICO Start Time: {}", config.ico_start_time);
        msg!("ICO End Time: {}", config.ico_end_time);
        msg!("TGE Time: {}", config.tge_time);

        Ok(())
    }

    pub fn get_token_rate(ctx: Context<GetTokenRate>) -> Result<()> {
        let config = &ctx.accounts.ico_config;

        msg!("Token Rate per SOL: {}", config.tokens_per_sol);
        msg!("Token Rate per USDC: {}", config.tokens_per_usdc);
        msg!("Token Rate per USDT: {}", config.tokens_per_usdt);

        Ok(())
    }

    pub fn get_min_max_buy_amount(ctx: Context<GetMinMaxBuyAmount>) -> Result<()> {
        let config = &ctx.accounts.ico_config;

        msg!("Min Buy Amount: {}", config.min_amount);
        msg!("Max Buy Amount: {}", config.max_amount);

        Ok(())
    }

    pub fn get_seconds_per_day(ctx: Context<GetSecondsPerDay>) -> Result<()> {
        let config = &ctx.accounts.ico_config;

        msg!("Seconds Per Day: {}", config.seconds_per_day);

        Ok(())
    }

    pub fn get_sol_balance(ctx: Context<GetSOLBalance>) -> Result<()> {
        let balance = ctx.accounts.sol_treasury.lamports();
        msg!("SOL Treasury Address: {}", ctx.accounts.sol_treasury.key());
        msg!("SOL Balance (Lamports): {}", balance);
        Ok(())
    }

    pub fn get_token_balance(ctx: Context<GetTokenBalance>) -> Result<u64> {
        // Deserialize the TokenAccount from AccountInfo
        let token_account: TokenAccount = TokenAccount::try_deserialize(&mut &ctx.accounts.token_account.data.borrow()[..])?;
    
        msg!("Balance of token: {}", token_account.amount);
        // Return the balance
        Ok(token_account.amount)
    }

    pub fn get_investor_address(ctx: Context<GetInvestorAddress>) -> Result<()> {
        let investor_address = ctx.accounts.investor_details.address;
        msg!("Investor Address: {}", investor_address);
        Ok(())
    }

    // Function to get vesting balance information (read-only)
    pub fn get_vesting_balance(
        ctx: Context<GetVestingBalance>,
        investor_address: Pubkey,
    ) -> Result<()> {
        let config = &ctx.accounts.ico_config;
        let investor = &ctx.accounts.investor_details;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;
    
        // Check if investor exists
        require!(investor.allocation > 0, CustomError::InvestorNotFound);
    
        // Calculate claimable tokens based on vesting type
        let (claimable_tokens, total_releasable) = calculate_claimable_tokens(
            investor.vesting_type,
            investor.allocation,
            investor.claimed_tokens,
            config.tge_time,
            config.seconds_per_day,
            current_time,
        )?;
    
        // Calculate total allocated
        let total_allocated = investor.allocation;
    
        // Calculate total remaining
        let total_remaining = total_allocated.saturating_sub(investor.claimed_tokens);
    
        // Get next cliff end date
        let next_cliff = get_next_cliff_end(
            investor.vesting_type,
            investor.cliff_end,
            config.seconds_per_day,
            config.tge_time,
            current_time,
        )?;
    
        // Log all information using msg! macros
        msg!("===== Vesting Balance Information =====");
        msg!("Investor Address: {}", investor.address);
        msg!("Vesting Type: {}", investor.vesting_type);
        msg!("Total Allocation: {}", total_allocated);
        msg!("Claimed Tokens: {}", investor.claimed_tokens);
        msg!("Claimable Now: {}", claimable_tokens);
        msg!("Total Vested: {}", total_releasable);
        msg!("Remaining Tokens: {}", total_remaining);
        msg!("Next Cliff Timestamp: {}", next_cliff);
        msg!("Last Claimed Timestamp: {}", investor.last_claimed);
        msg!("Is Blocked: {}", investor.blocked);
        msg!("======================================");
    
        Ok(())
    }

        // Function to get linear vesting end time (read-only)
    pub fn get_linear_vesting_end_time(
        ctx: Context<GetLinearVestingEndTime>,
    ) -> Result<()> {
        let config = &ctx.accounts.ico_config;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;
            
        // Calculate month in seconds
        let month_in_seconds = config.seconds_per_day
            .checked_mul(30)
            .ok_or(CustomError::ArithmeticOverflow)?;
            
        // Calculate milestone timestamps
        let tge_timestamp = config.tge_time;
            
        // Define milestones
        let milestone_8_months = tge_timestamp.checked_add(
            month_in_seconds
                .checked_mul(8)
                .ok_or(CustomError::ArithmeticOverflow)?
            ).ok_or(CustomError::ArithmeticOverflow)?;
            
        let milestone_18_months = tge_timestamp.checked_add(
            month_in_seconds
                .checked_mul(18)
                .ok_or(CustomError::ArithmeticOverflow)?
            ).ok_or(CustomError::ArithmeticOverflow)?;
            
        let milestone_24_months = tge_timestamp.checked_add(
            month_in_seconds
                .checked_mul(24)
                .ok_or(CustomError::ArithmeticOverflow)?
            ).ok_or(CustomError::ArithmeticOverflow)?;
            
        let milestone_36_months = tge_timestamp.checked_add(
            month_in_seconds
                .checked_mul(36)
                .ok_or(CustomError::ArithmeticOverflow)?
            ).ok_or(CustomError::ArithmeticOverflow)?;
            
        let is_completed = current_time >= milestone_36_months;
            
        // Log all information using msg! macros
        msg!("===== Linear Vesting Schedule =====");
        msg!("TGE Timestamp: {}", tge_timestamp);
        msg!("8-Month Milestone (15%): {}", milestone_8_months);
        msg!("18-Month Milestone (35%): {}", milestone_18_months);
        msg!("24-Month Milestone (55%): {}", milestone_24_months);
        msg!("36-Month Milestone (100%): {}", milestone_36_months);
        msg!("Vesting Completed: {}", is_completed);
        msg!("==================================");
            
        Ok(())
    }
}

// Add this private helper function to handle vesting cliff logic
fn get_next_cliff_end(
    vesting_type: u8,
    last_cliff_end: u64,
    seconds_per_day: u64,
    tge_date: u64,
    current_time: u64,
) -> Result<u64> {
    match vesting_type {
        VESTING_TYPE_IMMEDIATE => {
            // Days passed since last cliff end
            let days_passed = if current_time <= last_cliff_end {
                0
            } else {
                current_time
                    .checked_sub(last_cliff_end)
                    .ok_or(CustomError::ArithmeticOverflow)?
                    .checked_div(seconds_per_day)
                    .ok_or(CustomError::ArithmeticOverflow)?
            };

            println!("days_passed =============> {:?}", days_passed);

            // Cap at 365 days
            // To get the next cliff, increased by 1
            let effective_days = std::cmp::min(days_passed + 1, 365);
            println!("effective_days =============> {:?}", effective_days);

            let total_seconds = seconds_per_day
                .checked_mul(effective_days)
                .ok_or(CustomError::ArithmeticOverflow)?;

            last_cliff_end
                .checked_add(total_seconds)
                .ok_or(CustomError::ArithmeticOverflow.into())
        }

        VESTING_TYPE_LINEAR => {
            let month_in_seconds = seconds_per_day
                .checked_mul(30)
                .ok_or(CustomError::ArithmeticOverflow)?;

            const MILESTONE_PERIODS: [u64; 4] = [8, 10, 6, 12];
            let mut cumulative_months = 0;

            for &period in &MILESTONE_PERIODS {
                cumulative_months += period;

                let milestone_seconds = month_in_seconds
                    .checked_mul(cumulative_months)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                let milestone_date = tge_date
                    .checked_add(milestone_seconds)
                    .ok_or(CustomError::ArithmeticOverflow)?;

                if milestone_date > current_time {
                    return Ok(milestone_date);
                }
            }

            // If passed all milestones
            let total_months: u64 = MILESTONE_PERIODS.iter().sum();
            let final_seconds = month_in_seconds
                .checked_mul(total_months)
                .ok_or(CustomError::ArithmeticOverflow)?;

            tge_date
                .checked_add(final_seconds)
                .ok_or(CustomError::ArithmeticOverflow.into())
        }

        _ => Err(CustomError::InvalidVestingType.into()),
    }
}

// Add this private helper function to handle whitelisting logic
fn process_investor_whitelist(
    investor: &mut Account<'_, Investor>,
    buyer_key: Pubkey,
    config_tge_time: u64,
    vesting_type: u8,
    tokens_to_allocate: u64,
) -> Result<bool> {
    let is_first_purchase = investor.allocation == 0;

    // If this is investor's first purchase, initialize all whitelist fields
    if is_first_purchase {
        investor.address = buyer_key;
        investor.vesting_type = vesting_type;
        investor.released_tokens = 0;
        investor.cliff_end = config_tge_time;
        investor.last_claimed = 0;
        investor.claimed_tokens = 0;
        investor.blocked = false;
        investor.whitelisted_at = Clock::get()?.unix_timestamp as u64;

        // Log initial whitelist
        msg!("Investor automatically whitelisted on first purchase!");
        msg!("Investor Address: {}", investor.address);
        msg!("Vesting Type: {:?}", investor.vesting_type);
        msg!("Whitelisted At: {}", investor.whitelisted_at);

        emit!(InvestorWhitelisted {
            investor: investor.address,
            allocation: tokens_to_allocate,
            vesting_type: investor.vesting_type,
            cliff_end: investor.cliff_end,
        });
    }

    Ok(is_first_purchase)
}

// Helper function to calculate claimable tokens based on vesting type
fn calculate_claimable_tokens(
    vesting_type: u8,
    total_allocation: u64,
    already_claimed: u64,
    tge_time: u64,
    seconds_per_day: u64,
    current_time: u64,
) -> Result<(u64, u64)> {
    // Calculate the total releasable tokens based on vesting type and current time
    let total_releasable = match vesting_type {
        // Immediate vesting - 10% at TGE, rest linearly over 365 days
        VESTING_TYPE_IMMEDIATE => {
            // 10% at TGE initially

            let mut initial_release = total_allocation
                .checked_mul(10)
                .ok_or(CustomError::ArithmeticOverflow)?
                .checked_div(100)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Remaining 90% for daily vesting
            let mut remaining_allocation = total_allocation
                .checked_sub(initial_release)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Daily vesting amount
            let daily_release = remaining_allocation
                .checked_div(365)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Days passed since TGE
            let mut days_passed = if current_time <= tge_time {
                0
            } else {
                current_time
                    .checked_sub(tge_time)
                    .ok_or(CustomError::ArithmeticOverflow)?
                    .checked_div(seconds_per_day)
                    .ok_or(CustomError::ArithmeticOverflow)?
            };

            msg!("days_passed =============> {:?}", days_passed);

            // Cap at 365 days
            let effective_days = std::cmp::min(days_passed, 365);
            msg!("effective_days =============> {:?}", effective_days);

            if effective_days == 365 {
                total_allocation
            } else {
            // Calculate vested tokens
            let vested_from_daily = daily_release
                .checked_mul(effective_days)
                .ok_or(CustomError::ArithmeticOverflow)?;

            msg!("vested_from_daily =============> {:?}", vested_from_daily);

            // Total releasable: initial + vested from daily
            initial_release
                .checked_add(vested_from_daily)
                .ok_or(CustomError::ArithmeticOverflow)?
            }
        }

        // Linear vesting with milestones
        VESTING_TYPE_LINEAR => {
            // Initial 5% at TGE
            let mut initial_release = total_allocation
                .checked_mul(5)
                .ok_or(CustomError::ArithmeticOverflow)?
                .checked_div(100)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Calculate seconds in months for different milestones
            let mut month_in_seconds = seconds_per_day
                .checked_mul(30)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Define milestones in seconds from TGE
            let mut milestone_8_months = month_in_seconds
                .checked_mul(8)
                .ok_or(CustomError::ArithmeticOverflow)?;
            let mut milestone_18_months = month_in_seconds
                .checked_mul(18)
                .ok_or(CustomError::ArithmeticOverflow)?;
            let mut milestone_24_months = month_in_seconds
                .checked_mul(24)
                .ok_or(CustomError::ArithmeticOverflow)?;
            let mut milestone_36_months = month_in_seconds
                .checked_mul(36)
                .ok_or(CustomError::ArithmeticOverflow)?;

            // Calculate time elapsed since TGE
            let mut time_elapsed = if current_time <= tge_time {
                0
            } else {
                current_time
                    .checked_sub(tge_time)
                    .ok_or(CustomError::ArithmeticOverflow)?
            };

            // Calculate vested amount based on time passed
            let mut vested_amount = initial_release; // Start with TGE release (5%)

            // TGE + 8 months: 10%
            if time_elapsed >= milestone_8_months {
                vested_amount = vested_amount
                    .checked_add(
                        total_allocation
                            .checked_mul(10)
                            .ok_or(CustomError::ArithmeticOverflow)?
                            .checked_div(100)
                            .ok_or(CustomError::ArithmeticOverflow)?,
                    ).ok_or(CustomError::ArithmeticOverflow)?;
            }

            // TGE + 18 months: 20%
            if time_elapsed >= milestone_18_months {
                vested_amount = vested_amount
                    .checked_add(
                        total_allocation
                            .checked_mul(20)
                            .ok_or(CustomError::ArithmeticOverflow)?
                            .checked_div(100)
                            .ok_or(CustomError::ArithmeticOverflow)?,
                    ).ok_or(CustomError::ArithmeticOverflow)?;
            }

            // TGE + 24 months: 20%
            if time_elapsed >= milestone_24_months {
                vested_amount = vested_amount
                    .checked_add(
                        total_allocation
                            .checked_mul(20)
                            .ok_or(CustomError::ArithmeticOverflow)?
                            .checked_div(100)
                            .ok_or(CustomError::ArithmeticOverflow)?,
                    ).ok_or(CustomError::ArithmeticOverflow)?;
            }

            // TGE + 36 months: 45% (remaining all)
            if time_elapsed >= milestone_36_months {
                vested_amount = total_allocation;
            }

            vested_amount
        }
        _ => return Err(CustomError::InvalidVestingType.into()),
    };

    // Calculate claimable tokens (total releasable minus already claimed)
    msg!("already_claimed ===========> {}", already_claimed);
    msg!("total_releasable ===========> {}", total_releasable);
    let claimable = total_releasable.saturating_sub(already_claimed);
    msg!("claimable ===========> {}", claimable);

    Ok((claimable, total_releasable))
}

// =============================== Accounts ======================================

#[derive(Accounts)]
pub struct InitializeIco<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = ANCHOR_DISCRIMINATOR_SIZE + size_of::<TokenIco>(),
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    pub reward_token_mint: Account<'info, Mint>,

    pub usdc_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
    
    /// CHECK: This is the treasury account where SOL is stored. Owner check is done implicitly by transfer.
    #[account(mut)]
    pub sol_treasury: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = usdc_treasury.mint == usdc_mint.key(),
        constraint = usdc_treasury.owner == authority.key(),
    )]
    pub usdc_treasury: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = usdt_treasury.mint == usdt_mint.key(),
        constraint = usdt_treasury.owner == authority.key(),
    )]
    pub usdt_treasury: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = reward_token_treasury.mint == reward_token_mint.key(),
        constraint = reward_token_treasury.owner == vault_authority.key(),
    )]
    pub reward_token_treasury: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(investor_address:Pubkey)]
pub struct InvestorEntry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        init_if_needed,
        payer = authority,
        space = ANCHOR_DISCRIMINATOR_SIZE + size_of::<Investor>(),
        seeds = [b"investor is my hero", investor_address.key().as_ref()],
        bump
    )]
    pub investor_details: Account<'info, Investor>,

    pub system_program: Program<'info, System>,
}

// For BuyTokensWithSol
#[derive(Accounts)]
#[instruction(investor_address: Pubkey, amount: u64, vesting_type: u8)]
pub struct BuyTokensWithSol<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        init_if_needed,  // This allows initialization if the account doesn't exist
        payer = buyer,   // Buyer pays for account creation
        space = ANCHOR_DISCRIMINATOR_SIZE + size_of::<Investor>(),
        seeds = [b"investor is my hero", investor_address.as_ref()],
        bump
    )]
    pub investor_details: Account<'info, Investor>,

    /// CHECK: Validated against config.sol_treasury
    #[account(mut, address = ico_config.sol_treasury)]
    pub sol_treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// For BuyTokenWithUsdc
#[derive(Accounts)]
#[instruction(investor_address: Pubkey, amount: u64, vesting_type: u8)]
pub struct BuyTokenWithUsdc<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        init_if_needed,  // This allows initialization if the account doesn't exist
        payer = buyer,   // Buyer pays for account creation
        space = ANCHOR_DISCRIMINATOR_SIZE + size_of::<Investor>(),
        seeds = [b"investor is my hero", investor_address.as_ref()],
        bump
    )]
    pub investor_details: Account<'info, Investor>,

    #[account(
        mut,
        constraint = usdc_treasury.mint == usdc_mint.key(),
        address = ico_config.usdc_treasury
    )]
    pub usdc_treasury: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = buyer_token_account.owner == buyer.key(), 
        constraint = buyer_token_account.mint == usdc_mint.key()
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    #[account(address = ico_config.usdc_mint)]
    pub usdc_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(investor_address: Pubkey, amount: u64, vesting_type: u8)]
pub struct BuyTokenWithUsdt<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        init_if_needed,  // This allows initialization if the account doesn't exist
        payer = buyer,   // Buyer pays for account creation
        space = ANCHOR_DISCRIMINATOR_SIZE + size_of::<Investor>(),
        seeds = [b"investor is my hero", investor_address.as_ref()],
        bump
    )]
    pub investor_details: Account<'info, Investor>,

    #[account(
        mut,
        constraint = usdt_treasury.mint == usdt_mint.key(),
        address = ico_config.usdt_treasury
    )]
    pub usdt_treasury: Account<'info, TokenAccount>,

    #[account(
        mut, 
        constraint = buyer_token_account.owner == buyer.key(), 
        constraint = buyer_token_account.mint == usdt_mint.key()
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    #[account(address = ico_config.usdt_mint)]
    pub usdt_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Calculate<'info> {
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(investor_address: Pubkey)]
pub struct ClaimTokens<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut,
        seeds = [b"investor is my hero", investor_address.as_ref()],
        bump,
        // constraint = !investor_details.blocked @ CustomError::InvestorIsBlocked,
    )]
    pub investor_details: Account<'info, Investor>,

    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        constraint = reward_token_treasury.mint == ico_config.reward_token_mint,
        constraint = reward_token_treasury.owner == vault_authority.key()
    )]
    pub reward_token_treasury: Account<'info, TokenAccount>,

    /// Authority that can sign for token transfers from treasury
    #[account(
        // address = ico_config.authority @ CustomError::UnauthorizedAccess
        mut
    )]
    pub investor: Signer<'info>,

    #[account(
        mut,
        constraint = investor_token_account.mint == ico_config.reward_token_mint,
        constraint = investor_token_account.owner == investor.key()
    )]
    pub investor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetIcoConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump,
        constraint = ico_config.authority == authority.key()
    )]
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct SetMintAddress<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump,
        constraint = ico_config.authority == authority.key()
    )]
    pub ico_config: Account<'info, TokenIco>,

    /// CHECK: Used only to fetch decimals from the mint; verified by CPI
    pub mint_account: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump,
        constraint = ico_config.authority == authority.key()
    )]
    pub ico_config: Account<'info, TokenIco>,
    pub authority: Signer<'info>,
}


#[derive(Accounts)]
pub struct UpdateTreasuryWallet<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump,
        constraint = authority.key() == ico_config.authority @ CustomError::UnauthorizedAccess
    )]
    pub ico_config: Account<'info, TokenIco>,

    /// CHECK: This is explicitly checked to be owned by the System Program
    #[account(
        constraint = *new_wallet.owner == system_program.key() @ CustomError::InvalidWalletType
    )]
    pub new_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(investor_address:Pubkey)]
pub struct RemoveInvestor<'info> {
    #[account(mut)]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut, 
        seeds = [b"investor is my hero", investor_address.key().as_ref()],
        bump,
        close = authority
    )]
    pub investor_details: Account<'info, Investor>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawSol<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut,
        address = ico_config.sol_treasury @ CustomError::InvalidTreasury,
    )]

    /// CHECK: This is the treasury account from which SOL will be withdrawn
    pub sol_treasury: Signer<'info>,

    // /// The authority that can withdraw (must be signer)
    // #[account(
    //     address = ico_config.authority @ CustomError::UnauthorizedAccess
    // )]
    // pub authority: Signer<'info>,
    /// CHECK: This is the recipient account to which SOL will be transferred
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PrizeTokensTransfer<'info> {
    #[account(
        constraint = ico_config.authority == authority.key()
    )]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut,
        constraint = recipient_token_account.mint == ico_config.reward_token_mint,
        constraint = recipient_token_account.owner == authority.key()
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        constraint = reward_token_treasury.mint == ico_config.reward_token_mint,
        constraint = reward_token_treasury.owner == vault_authority.key()
    )]
    pub reward_token_treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct UsdcTokensTransfer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut,
        constraint = recipient_usdc_account.mint == ico_config.usdc_mint,
    )]
    pub recipient_usdc_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = usdc_treasury.mint == ico_config.usdc_mint.key(),
        constraint = usdc_treasury.owner == authority.key()
    )]
    pub usdc_treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UsdtTokensTransfer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut,
        constraint = recipient_usdt_account.mint == ico_config.usdt_mint,
    )]
    pub recipient_usdt_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = usdt_treasury.mint == ico_config.usdt_mint.key(),
        constraint = usdt_treasury.owner == authority.key(),
        
    )]
    pub usdt_treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(investor_address:Pubkey)]
pub struct BlockInvestor<'info> {
    pub ico_config: Account<'info, TokenIco>,

    #[account(
        mut, 
        seeds = [b"investor is my hero", investor_address.key().as_ref()],
        bump
    )]
    pub investor_details: Account<'info, Investor>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositPrize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,

    #[account(mut, address = ico_config.reward_token_mint)]
    pub reward_token_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = authority_token_account.owner == authority.key(),
        constraint = authority_token_account.mint == reward_token_mint.key()
    )]
    pub authority_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"vault_authority"],
        bump
    )]
    pub vault_authority: AccountInfo<'info>,

    #[account(
        mut,
        constraint = reward_token_treasury.mint == ico_config.reward_token_mint,
        constraint = reward_token_treasury.owner == vault_authority.key()
    )]
    pub reward_token_treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct RenounceOwnership<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump
    )]
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct GetIcoDates<'info> {
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct GetTokenRate<'info> {
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct GetMinMaxBuyAmount<'info> {
    pub ico_config: Account<'info, TokenIco>,
}


#[derive(Accounts)]
pub struct GetSecondsPerDay<'info> {
    pub ico_config: Account<'info, TokenIco>,
}

#[derive(Accounts)]
pub struct GetSOLBalance<'info> {
    /// CHECK: Only reading lamports, safe
    pub sol_treasury: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct GetTokenBalance<'info> {
    /// Token account, can be dynamically loaded
    pub token_account: AccountInfo<'info>,
}

#[derive(Accounts)]
// #[instruction(investor_address: Pubkey)]
pub struct GetInvestorAddress<'info> {
    pub investor_details: Account<'info, Investor>,
}

// Define the context for GetVestingBalance
#[derive(Accounts)]
#[instruction(investor_address: Pubkey)]
pub struct GetVestingBalance<'info> {
    pub ico_config: Account<'info, TokenIco>,
    
    #[account(
        seeds = [b"investor is my hero", investor_address.as_ref()],
        bump,
    )]
    pub investor_details: Account<'info, Investor>,
    
    pub system_program: Program<'info, System>,
}


// Define the context for GetLinearVestingEndTime
#[derive(Accounts)]
pub struct GetLinearVestingEndTime<'info> {
    #[account(mut)]
    pub ico_config: Account<'info, TokenIco>,
    
    pub system_program: Program<'info, System>,
}


// ========================= Account ===================================

// Token Ico Configuration
#[account]
pub struct TokenIco {
    pub authority: Pubkey,
    pub reward_token_mint: Pubkey,

    pub sol_treasury: Pubkey,
    pub usdc_treasury: Pubkey,
    pub usdt_treasury: Pubkey,
    pub prize_treasury: Pubkey,

    pub usdc_mint: Pubkey,
    pub usdt_mint: Pubkey,

    pub usdc_decimals: u8,
    pub usdt_decimals: u8,
    pub reward_token_decimals: u8,

    // pub prize_mint: Pubkey,
    pub ico_start_time: u64,
    pub ico_end_time: u64,
    pub tge_time: u64,

    pub min_amount: u64,
    pub max_amount: u64,

    pub tokens_per_sol: u64,
    pub tokens_per_usdc: u64,
    pub tokens_per_usdt: u64,

    pub seconds_per_day: u64,

    pub paused: bool,

    pub total_user_allocated: u64,   // Allocated through purchases
    pub total_prize_deposited: u64,  // Deposited as prize tokens
    // pub whitelist_required: bool,
    pub total_allocated: u64,
    // pub total_sold: u64,
    pub total_claimed: u64,
}

#[account]
pub struct Investor {
    pub address: Pubkey,
    pub allocation: u64,
    pub vesting_type: u8,
    pub released_tokens: u64,
    pub cliff_end: u64,
    pub last_claimed: u64,
    pub claimed_tokens: u64,
    pub blocked: bool, // Added field to indicate if the investor is blocked
    pub whitelisted_at: u64,
}

// all types of events

#[event]
pub struct InitializeEvent {
    pub authority: Pubkey,
    pub ico_start_time: u64,
    pub ico_end_time: u64,
    pub tge_time: u64,
}

#[event]
pub struct InvestorWhitelisted {
    pub investor: Pubkey,
    pub allocation: u64,
    pub vesting_type: u8,
    pub cliff_end: u64,
}

#[event]
pub struct TokenPurchaseEventForSol {
    pub buyer: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub timestamp: u64,
}

#[event]
pub struct TokenPurchaseEventForUsdc {
    pub buyer: Pubkey,
    pub usdc_amount: u64,
    pub token_amount: u64,
    pub timestamp: u64,
    pub usdc_treasury: Pubkey,
}

#[event]
pub struct TokenPurchaseEventForUsdt {
    pub buyer: Pubkey,
    pub usdt_amount: u64,
    pub token_amount: u64,
    pub timestamp: u64,
    pub usdt_treasury: Pubkey,
}

#[event]
pub struct ICODateChanged {
    pub authority: Pubkey,
    pub ico_start_time: u64,
    pub ico_end_time: u64,
    pub timestamp: u64,
}

#[event]
pub struct InvestorBlocked {
    pub investor: Pubkey,
    pub blocked_by: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct InvestorRemoved {
    pub investor: Pubkey,
    pub removed_by: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct PrizeTokenUpdated {
    pub authority: Pubkey,
    pub token: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct WithdrawnSol {
    pub authority: Pubkey,
    pub amount: u64,
    pub recipient: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct PrizeWithdrawTokens {
    pub authority: Pubkey,
    pub amount: u64,
    pub timestamp: u64,
}

#[event]
pub struct UsdcWithdrawTokens {
    pub authority: Pubkey,
    pub amount: u64,
    pub timestamp: u64,
}

#[event]
pub struct UsdtWithdrawTokens {
    pub authority: Pubkey,
    pub amount: u64,
    pub timestamp: u64,
}

#[event]
pub struct TokensClaimed {
    pub investor: Pubkey,
    pub amount: u64,
    pub timestamp: u64,
    pub remaining: u64,
}

#[event]
pub struct PrizeDeposited {
    pub depositor: Pubkey,
    pub amount: u64,
    pub timestamp: u64,
}

#[event]
pub struct UsdtAddressUpdated {
    pub authority: Pubkey,
    pub token: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct SolTreasuryUpdated {
    pub authority: Pubkey,
    pub sol_treasury: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct UsdcAddressUpdated {
    pub authority: Pubkey,
    pub token: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct TGEDateChanged {
    pub authority: Pubkey,
    pub new_tge_time: u64,
    pub timestamp: u64,
}

#[event]
pub struct SecondsPerDayChanged {
    pub authority: Pubkey,
    pub new_value: u64,
    pub old_value: u64,
    pub timestamp: u64,
}

#[event]
pub struct OwnershipTransferred {
    pub previous_owner: Pubkey,
    pub new_owner: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct OwnershipRenounced {
    pub previous_owner: Pubkey,
    pub timestamp: u64,
}

#[event]
pub struct PausedEvent {
    pub authority: Pubkey,
    pub paused: bool,
}


// ============================== custom types =====================

// Additional error codes
#[error_code]
pub enum CustomError {
    #[msg("The provided vesting type is not supported.")]
    InvalidVestingType,
    #[msg("ICO is paused.")]
    ICOIsPaused,
    #[msg("ICO has not started or has ended.")]
    ICOPhaseInvalid,
    #[msg("The purchase amount is below the minimum or above the maximum allowed.")]
    InvalidBuyAmount,
    #[msg("Calculation overflow occurred.")]
    CalculationOverflow,
    #[msg("Unauthorized access. Only admin can perform this action.")]
    UnauthorizedAccess,
    #[msg("Investor not found in whitelist.")]
    InvestorNotFound,
    #[msg("Invalid token type specified.")]
    InvalidTokenType,
    #[msg("Insufficient funds for withdrawal.")]
    InsufficientFunds,
    // #[msg("Min value must be less than or equal to max value.")]
    // InvalidMinMaxValues,
    #[msg("Invalid address provided.")]
    InvalidAddress,
    #[msg("Investor is blocked.")]
    InvestorIsBlocked,
    #[msg("TGE date is invalid.")]
    TGEDateInvalid,
    #[msg("No Token Available to claim")]
    NoTokensAvailableToClaim,
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    #[msg("Invalid Treasury wallet type. Must be a system-owned wallet.")]
    InvalidWalletType,
    // #[msg("Wallet account info must be provided as a remaining account.")]
    // WalletInfoNotProvided,
    #[msg("You assign invalid treasury")]
    InvalidTreasury,
    #[msg("Mismatched vesting type for existing investor.")]
    MismatchedVestingType,
    #[msg("Buyer have not enough rent exempt")]
    BuyerNotRentExempt
}

