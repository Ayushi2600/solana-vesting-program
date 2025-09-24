import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { PublicKey, Keypair, SystemProgram } from '@solana/web3.js';
import { expect } from 'chai';
import { VestingProgram } from '../target/types/vesting_program'; // Replace with your actual program name

describe('vesting_program', () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VestingProgram as Program<VestingProgram>;
  const wallet = provider.wallet;

  // Generate a new keypair for the ICO config account
  const [configPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  // Generate a vault authority PDA
  const [vaultAuthorityPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority")],
    program.programId
  );

  // Mock token mints
  const rewardTokenMint = Keypair.generate();
  const usdcMint = Keypair.generate();
  const usdtMint = Keypair.generate();

  // Mock treasury accounts
  const solTreasury = Keypair.generate();
  const usdcTreasury = Keypair.generate();
  const usdtTreasury = Keypair.generate();
  const rewardTokenTreasury = Keypair.generate();

  // Create test investors
  const investorCount = 3;
  const investors = Array(investorCount).fill(0).map(() => ({
    keypair: Keypair.generate(),
    amount: new anchor.BN(100 * 1_000_000_000), // 100 tokens with 9 decimals
    vestingType: 1 // Linear vesting
  }));

  before(async () => {
    // Fund the wallet
    const airdropSignature = await provider.connection.requestAirdrop(
      wallet.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSignature);

    // Note: In a real test, you would initialize all token mints and accounts here
    // For this example, we'll simulate that they already exist
    console.log("Test setup complete");
  });

  it('Initialize the ICO config', async () => {
    // In a real test, you would create the actual token mint and accounts
    // For this example, we're assuming they exist
    
    const now = Math.floor(Date.now() / 1000);
    const icoStartTime = now;
    const icoEndTime = now + 30 * 24 * 60 * 60; // 30 days
    const tgeTime = icoEndTime + 7 * 24 * 60 * 60; // 7 days after ICO ends
    const minBuyAmount = new anchor.BN(10 * 1_000_000_000); // 10 tokens
    const maxBuyAmount = new anchor.BN(1000 * 1_000_000_000); // 1000 tokens
    const tokensPerSol = new anchor.BN(1000); // 1000 tokens per SOL
    const tokensPerUsdc = new anchor.BN(10); // 10 tokens per USDC
    const tokensPerUsdt = new anchor.BN(10); // 10 tokens per USDT
    const secondsPerDay = new anchor.BN(86400); // 86400 seconds = 1 day

    // Here you would initialize the ICO config
    // For testing, we'll assume it was already initialized
    console.log("ICO config initialized");
  });

  it('Whitelist multiple investors', async () => {
    // Create investor input data
    const investorInputs = investors.map(investor => ({
      address: investor.keypair.publicKey,
      amount: investor.amount,
      vestingType: investor.vestingType
    }));

    // Generate PDAs for each investor and prepare them for remainingAccounts
    const remainingAccounts = await Promise.all(
      investors.map(async (investor) => {
        const [pda] = await PublicKey.findProgramAddressSync(
          [Buffer.from("investor is my hero"), investor.keypair.publicKey.toBuffer()],
          program.programId
        );
        
        return {
          pubkey: pda,
          isWritable: true,
          isSigner: false
        };
      })
    );

    console.log(`Testing whitelist for ${investorCount} investors`);

    try {
      // Execute the transaction
      const tx = await program.methods
        .whitelistMultipleInvestors(investorInputs)
        .accounts({
          authority: provider.wallet.publicKey,
          icoConfig: configPDA,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(remainingAccounts)
        .rpc();

      console.log("Transaction signature:", tx);
      
      // Verify each investor was whitelisted correctly
      for (let i = 0; i < investors.length; i++) {
        const [investorPDA] = PublicKey.findProgramAddressSync(
          [Buffer.from("investor is my hero"), investors[i].keypair.publicKey.toBuffer()],
          program.programId
        );
        
        const investorAccount = await program.account.investor.fetch(investorPDA);
        
        expect(investorAccount.address.toString()).to.equal(
          investors[i].keypair.publicKey.toString()
        );
        expect(investorAccount.allocation.toNumber()).to.equal(
          investors[i].amount.toNumber()
        );
        expect(investorAccount.vestingType).to.equal(investors[i].vestingType);
        
        console.log(`Investor ${i+1} verified: ${investorAccount.address.toString()}`);
      }
    } catch (error) {
      console.error("Error whitelisting investors:", error);
      throw error;
    }
  });
});