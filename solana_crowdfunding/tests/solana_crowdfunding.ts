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

  const [receipt1PDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("receipt"), campaignPDA.toBuffer(), donor1.publicKey.toBuffer()],
    program.programId
  );

  const [receipt2PDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("receipt"), campaignPDA.toBuffer(), donor2.publicKey.toBuffer()],
    program.programId
  );

  before(async () => {
    // Beri modal awal untuk donor
    const sig1 = await provider.connection.requestAirdrop(donor1.publicKey, 2000 * 1e9);
    const sig2 = await provider.connection.requestAirdrop(donor2.publicKey, 2000 * 1e9);
    await provider.connection.confirmTransaction(sig1);
    await provider.connection.confirmTransaction(sig2);
  });

  it("1. Create a campaign with goal=1000 SOL, deadline=tomorrow (simulated 3s)", async () => {
    const goal = new anchor.BN(1000 * 1e9);
    // Simulasi 'tomorrow' dengan 3 detik agar bisa di-test tanpa menunggu besok
    const now = Math.floor(Date.now() / 1000);
    const deadline = new anchor.BN(now + 3); 

    await program.methods.createCampaign(goal, deadline)
      .accounts({ creator: creator.publicKey })
      .rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.goal.toNumber()).to.equal(1000 * 1e9);
    expect(campaign.raised.toNumber()).to.equal(0);
  });

  it("2. Contribute 600 SOL -> should succeed, raised=600", async () => {
    const amount = new anchor.BN(600 * 1e9);
    
    await program.methods.contribute(amount)
      .accounts({ donor: donor1.publicKey, campaign: campaignPDA, receipt: receipt1PDA })
      .signers([donor1]).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.raised.toNumber()).to.equal(600 * 1e9);
  });

  it("3. Contribute 500 SOL -> should succeed, raised=1100", async () => {
    const amount = new anchor.BN(500 * 1e9);
    
    await program.methods.contribute(amount)
      .accounts({ donor: donor2.publicKey, campaign: campaignPDA, receipt: receipt2PDA })
      .signers([donor2]).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.raised.toNumber()).to.equal(1100 * 1e9);
  });

  it("4. Try withdraw before deadline -> should fail", async () => {
    try {
      await program.methods.withdraw()
        .accounts({ creator: creator.publicKey, campaign: campaignPDA }).rpc();
      expect.fail("Should have failed");
    } catch (e: any) {
      expect(e.error.errorMessage).to.equal("Campaign is still active.");
    }
  });

  it("5. Wait until after deadline -> withdraw should succeed", async () => {
    console.log("      (Waiting 4 seconds to simulate deadline passing...)");
    await new Promise((resolve) => setTimeout(resolve, 4000));

    await program.methods.withdraw()
      .accounts({ creator: creator.publicKey, campaign: campaignPDA }).rpc();

    const campaign = await program.account.campaign.fetch(campaignPDA);
    expect(campaign.claimed).to.be.true;
  });

  it("6. Try withdraw again -> should fail (already claimed)", async () => {
    try {
      await program.methods.withdraw()
        .accounts({ creator: creator.publicKey, campaign: campaignPDA }).rpc();
      expect.fail("Should have failed");
    } catch (e: any) {
      expect(e.error.errorMessage).to.equal("Funds already claimed.");
    }
  });
});