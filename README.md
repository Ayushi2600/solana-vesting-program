# Token Vesting ICO Program

A Solana-based Initial Coin Offering (ICO) program with token vesting capabilities, built using the Anchor framework. This program supports multiple payment methods (SOL, USDC, USDT) and flexible vesting schedules.

## Program ID
```
3XDaifPQuLu23EQ5SnQkkVssxDj4TfCApxtyxsKSA6GU
```

## Features

### Core Functionality
- **Multi-Currency Support**: Accept payments in SOL, USDC, and USDT
- **Flexible Vesting**: Two vesting types - Immediate and Linear
- **Admin Controls**: Comprehensive administrative functions
- **Whitelist Management**: Manual investor whitelisting by admin
- **Token Claims**: Automated vesting-based token distribution
- **Treasury Management**: Separate treasuries for each accepted currency

### Vesting Types

#### 1. Immediate Vesting (Type 0)
- **TGE Release**: 10% of tokens released immediately at Token Generation Event
- **Linear Vesting**: Remaining 90% vested daily over 365 days
- **Formula**: Daily release = (Total Allocation × 0.9) ÷ 365

#### 2. Linear Vesting (Type 1) - Milestone-Based
- **TGE**: 5% released immediately
- **8 Months**: Additional 10% (Total: 15%)
- **18 Months**: Additional 20% (Total: 35%)
- **24 Months**: Additional 20% (Total: 55%)
- **36 Months**: Remaining 45% (Total: 100%)

#### 3. Administrative Controls
- `set_ico_dates()` - Update ICO start/end times
- `set_tge_date()` - Update token generation event time
- `set_paused()` - Emergency pause/unpause
- `remove_investor()` - Remove investor and reclaim allocation
- `block_investor()` - Block investor from claiming
- `transfer_ownership()` - Transfer program authority
- `renounce_ownership()` - Renounce program authority

### Query Functions

- `get_ico_dates()` - Retrieve ICO timing information
- `get_token_rate()` - Get current exchange rates
- `get_min_max_buy_amount()` - Get purchase limits
- `get_vesting_balance()` - Check investor's vesting status
- `get_linear_vesting_end_time()` - Get linear vesting milestones
- `determine_claimable_tokens()` - Calculate claimable amount


## Security Features

1. **Authority Checks**: All admin functions require proper authority validation
2. **Time-based Controls**: ICO phases and vesting schedules are time-locked
3. **Overflow Protection**: All arithmetic operations include overflow checks
4. **Account Validation**: Strict account ownership and type validation
5. **Emergency Pause**: Admin can pause all ICO operations
6. **Investor Blocking**: Admin can block malicious investors

## Error Codes

- `InvalidVestingType` - Unsupported vesting type specified
- `ICOIsPaused` - Operations attempted while ICO is paused
- `ICOPhaseInvalid` - Purchase attempted outside ICO timeframe
- `InvalidBuyAmount` - Purchase amount outside allowed range
- `UnauthorizedAccess` - Non-admin attempting admin function
- `InvestorNotFound` - Investor not found in system
- `InvestorIsBlocked` - Blocked investor attempting operation
- `TGEDateInvalid` - Invalid Token Generation Event timing
- `NoTokensAvailableToClaim` - No vested tokens available
- `InsufficientFunds` - Treasury lacks sufficient tokens

## Development Setup

1. Install Anchor CLI
2. Clone the repository
3. Build the program:
```anchor build```
5. Deploy:
```anchor deploy```
7. Run tests:
```anchor test```

## Support

For technical support or questions about implementation, please refer to the Anchor documentation or Solana developer resources.
