import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaCrowdfunding } from "../target/types/solana_crowdfunding";
import { expect } from "chai";

describe("solana_crowdfunding", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaCrowdfunding as Program<SolanaCrowdfunding>;
  
  const creator = provider.wallet as anchor.Wallet;
  const donor1 = anchor.web3.Keypair.generate();
  const donor2 = anchor.web3.Keypair.generate();

  const [campaignPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("campaign"), creator.publicKey.toBuffer()],
    program.programId
  );

  const [vaultPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), campaignPDA.toBuffer()],
    program.programId
  );

  const [receipt1PDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("receipt"), campaignPDA.toBuffer(), donor1.publicKey.toBuffer()],
    program.programId
  );

  const [receipt2PDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("receipt"), campaignPDA.toBuffer(), donor2.publicKey.toBuffer()],
    program.programId
  );

  before(async () => {
    // Top up donors with SOL
    const sig1 = await provider.connection.requestAirdrop(donor1.publicKey, 2000 * 1e9);
    const sig2 = await provider.connection.requestAirdrop(donor2.publicKey, 2000 * 1e9);
    await provider.connection.confirmTransaction(sig1);
    await provider.connection.confirmTransaction(sig2);
  });

  it("1. Create a campaign with goal=1000 SOL, deadline=tomorrow (simulated 2s)", async () => {
    const goal = new anchor.BN(1000 * 1e9);
    const now = Math.floor(Date.now() / 1000);
    const deadline = new anchor.BN(now + 2); 

    await program.methods.createCampaign(goal, deadline)
      .accounts({ creator: creator.publicKey, campaign: campaignPDA })
      .rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.goal.toNumber()).to.equal(1000 * 1e9);
    expect(campaign.raised.toNumber()).to.equal(0);
  });

  it("2. Contribute 600 SOL -> should succeed, raised=600", async () => {
    const amount = new anchor.BN(600 * 1e9);
    
    await program.methods.contribute(amount)
      .accounts({ 
        donor: donor1.publicKey, 
        campaign: campaignPDA, 
        vault: vaultPDA,
        receipt: receipt1PDA 
      })
      .signers([donor1]).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.raised.toNumber()).to.equal(600 * 1e9);
    
    const vaultBalance = await provider.connection.getBalance(vaultPDA);
    expect(vaultBalance).to.be.at.least(600 * 1e9);
  });

  it("3. Contribute 500 SOL -> should succeed, raised=1100", async () => {
    const amount = new anchor.BN(500 * 1e9);
    
    await program.methods.contribute(amount)
      .accounts({ 
        donor: donor2.publicKey, 
        campaign: campaignPDA, 
        vault: vaultPDA,
        receipt: receipt2PDA 
      })
      .signers([donor2]).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.raised.toNumber()).to.equal(1100 * 1e9);

    const vaultBalance = await provider.connection.getBalance(vaultPDA);
    expect(vaultBalance).to.be.at.least(1100 * 1e9);
  });

  it("4. Try withdraw before deadline -> should fail", async () => {
    try {
      await program.methods.withdraw()
        .accounts({ 
          creator: creator.publicKey, 
          campaign: campaignPDA,
          vault: vaultPDA 
        }).rpc();
      expect.fail("Should have failed");
    } catch (e: any) {
      expect(e.error.errorMessage).to.equal("Campaign is still active.");
    }
  });

  it("5. Wait until after deadline -> withdraw should succeed", async () => {
    console.log("      (Waiting 3 seconds to simulate deadline passing...)");
    await new Promise((resolve) => setTimeout(resolve, 3000));

    const creatorBalanceBefore = await provider.connection.getBalance(creator.publicKey);

    await program.methods.withdraw()
      .accounts({ 
        creator: creator.publicKey, 
        campaign: campaignPDA,
        vault: vaultPDA 
      }).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.claimed).to.be.true;

    const creatorBalanceAfter = await provider.connection.getBalance(creator.publicKey);
    expect(creatorBalanceAfter).to.be.greaterThan(creatorBalanceBefore);
  });

  it("6. Try withdraw again -> should fail (already claimed)", async () => {
    try {
      await program.methods.withdraw()
        .accounts({ 
          creator: creator.publicKey, 
          campaign: campaignPDA,
          vault: vaultPDA 
        }).rpc();
      expect.fail("Should have failed");
    } catch (e: any) {
      expect(e.error.errorMessage).to.equal("Funds already claimed.");
    }
  });

  describe("Refund Scenarios (Campaign 2)", () => {
    let campaign2PDA: anchor.web3.PublicKey;
    let vault2PDA: anchor.web3.PublicKey;
    let receipt1_2PDA: anchor.web3.PublicKey;

    it("7. Create a new campaign for refund test (high goal)", async () => {
      const creator2 = anchor.web3.Keypair.generate();
      const sig = await provider.connection.requestAirdrop(creator2.publicKey, 10 * 1e9);
      await provider.connection.confirmTransaction(sig);

      [campaign2PDA] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("campaign"), creator2.publicKey.toBuffer()],
        program.programId
      );

      [vault2PDA] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), campaign2PDA.toBuffer()],
        program.programId
      );

      [receipt1_2PDA] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("receipt"), campaign2PDA.toBuffer(), donor1.publicKey.toBuffer()],
        program.programId
      );

      const goal = new anchor.BN(5000 * 1e9);
      const now = Math.floor(Date.now() / 1000);
      const deadline = new anchor.BN(now + 2);

      await program.methods.createCampaign(goal, deadline)
        .accounts({ creator: creator2.publicKey, campaign: campaign2PDA })
        .signers([creator2]).rpc();
    });

    it("8. Contribute 100 SOL (not enough for goal)", async () => {
      const amount = new anchor.BN(100 * 1e9);
      await program.methods.contribute(amount)
        .accounts({ 
          donor: donor1.publicKey, 
          campaign: campaign2PDA, 
          vault: vault2PDA,
          receipt: receipt1_2PDA 
        })
        .signers([donor1]).rpc();
    });

    it("9. Wait for deadline and claim refund", async () => {
      console.log("      (Waiting 3 seconds to simulate deadline passing...)");
      await new Promise((r) => setTimeout(r, 3000));

      const donorBalanceBefore = await provider.connection.getBalance(donor1.publicKey);
      
      await program.methods.refund()
        .accounts({ 
          donor: donor1.publicKey, 
          campaign: campaign2PDA, 
          vault: vault2PDA,
          receipt: receipt1_2PDA 
        })
        .signers([donor1]).rpc();

      const donorBalanceAfter = await provider.connection.getBalance(donor1.publicKey);
      expect(donorBalanceAfter).to.be.greaterThan(donorBalanceBefore);

      const receipt = await program.account.receipt.fetch(receipt1_2PDA);
      expect(receipt.amount.toNumber()).to.equal(0);
    });
  });
});