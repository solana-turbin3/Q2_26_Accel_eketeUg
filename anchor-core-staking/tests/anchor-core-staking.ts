import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorCoreStaking } from "../target/types/anchor_core_staking";
import { SystemProgram } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

const MILLISECONDS_PER_DAY = 86400000;
const REWARDS_BPS = 10000;
const FREEZE_PERIOD_IN_DAYS = 7;
const TIME_TRAVEL_IN_DAYS = 8;

function getAttribute(buffer: Buffer, key: string): string | null {
  const keyBytes = Buffer.from(key, "utf-8");
  const lenBytes = Buffer.alloc(4);
  lenBytes.writeUInt32LE(keyBytes.length, 0);
  
  const searchPattern = Buffer.concat([lenBytes, keyBytes]);
  const index = buffer.indexOf(searchPattern);
  if (index === -1) {
    return null;
  }
  
  const valueStart = index + searchPattern.length;
  if (valueStart + 4 > buffer.length) {
    return null;
  }
  
  const valueLen = buffer.readUInt32LE(valueStart);
  if (valueStart + 4 + valueLen > buffer.length) {
    return null;
  }
  
  return buffer.slice(valueStart + 4, valueStart + 4 + valueLen).toString("utf-8");
}

describe("anchor-core-staking", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.anchorCoreStaking as Program<AnchorCoreStaking>;

  // Generate a keypair for the collection
  const collectionKeypair = anchor.web3.Keypair.generate();

  // Find the update authority for the collection (PDA)
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Generate a keypair for the nft asset
  const nftKeypair = anchor.web3.Keypair.generate();

  // Find the config account (PDA)
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  // Find the rewards mint account (PDA)
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards_mint"), config.toBuffer()],
    program.programId
  )[0];

  // Helper function to advance time with Surfpool 
  async function advanceTime(params: { absoluteEpoch?: number; absoluteSlot?: number; absoluteTimestamp?: number }): Promise<void> {
    const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [params],
      }),
    });

    const result = await rpcResponse.json() as { error?: any; result?: any };
    if (result.error) {
      throw new Error(`Time travel failed: ${JSON.stringify(result.error)}`);
    }
    
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  it("Create a collection", async () => {
    const collectionName = "Test Collection";
    const collectionUri = "https://example.com/collection";
    const tx = await program.methods.createCollection(collectionName, collectionUri)
    .accountsPartial({
      payer: provider.wallet.publicKey,
      collection: collectionKeypair.publicKey,
      updateAuthority,
      systemProgram: SystemProgram.programId,
      mplCoreProgram: MPL_CORE_PROGRAM_ID,
    })
    .signers([collectionKeypair])
    .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Collection address", collectionKeypair.publicKey.toBase58());
  });

  it("Mint an NFT", async () => {
    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods.mintAsset(nftName, nftUri)
    .accountsPartial({
      user: provider.wallet.publicKey,
      asset: nftKeypair.publicKey,
      collection: collectionKeypair.publicKey,
      updateAuthority,
      systemProgram: SystemProgram.programId,
      mplCoreProgram: MPL_CORE_PROGRAM_ID,
    })
    .signers([nftKeypair])
    .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("NFT address", nftKeypair.publicKey.toBase58());
  });

  it("Initialize Config", async () => {
    const tx = await program.methods.initialize(REWARDS_BPS, FREEZE_PERIOD_IN_DAYS)
    .accountsPartial({
      admin: provider.wallet.publicKey,
      collection: collectionKeypair.publicKey,
      updateAuthority,
      config,
      rewardsMint,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Config address", config.toBase58());
    console.log("Rewards BPS", REWARDS_BPS);
    console.log("Freeze period in days", FREEZE_PERIOD_IN_DAYS);
    console.log("Rewards mint address", rewardsMint.toBase58());
  });

  it("Stake an NFT", async () => {
    const tx = await program.methods.stake()
    .accountsPartial({
      owner: provider.wallet.publicKey,
      updateAuthority,
      config,
      asset: nftKeypair.publicKey,
      collection: collectionKeypair.publicKey,
      systemProgram: SystemProgram.programId,
      mplCoreProgram: MPL_CORE_PROGRAM_ID,
    })
    .rpc();
    console.log("\nYour transaction signature", tx);

    // Verify statistics on collection
    const collectionInfo = await provider.connection.getAccountInfo(collectionKeypair.publicKey);
    const totalStakedStr = getAttribute(collectionInfo.data, "total_staked");
    console.log("Collection total staked:", totalStakedStr);
    anchor.assert.equal(totalStakedStr, "1");

    // Verify asset is staked
    const assetInfo = await provider.connection.getAccountInfo(nftKeypair.publicKey);
    const stakedStr = getAttribute(assetInfo.data, "staked");
    const stakedAtStr = getAttribute(assetInfo.data, "staked_at");
    const lastClaimedAtStr = getAttribute(assetInfo.data, "last_claimed_at");
    anchor.assert.equal(stakedStr, "true");
    anchor.assert.ok(stakedAtStr && stakedAtStr !== "0");
    anchor.assert.equal(lastClaimedAtStr, stakedAtStr);
  });

  it("Try to claim rewards immediately", async () => {
    const userRewardsAta = getAssociatedTokenAddressSync(rewardsMint, provider.wallet.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    try {
      await program.methods.claimRewards()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        asset: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
      throw new Error("Claim rewards should have failed immediately because 0 days elapsed");
    } catch (err) {
      if (err instanceof anchor.AnchorError && err.error.errorCode.code === "NoRewardsToClaim") {
        console.log("\nClaim rewards failed as expected:", err.error.errorMessage);
      } else {
        throw err;
      }
    }
  });

  it("Try to unstake an NFT before the freeze period ends", async () => {
    // Get the user rewards ATA account
    const userRewardsAta = getAssociatedTokenAddressSync(rewardsMint, provider.wallet.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    try {
      const tx = await program.methods.unstake()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        asset: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
      throw new Error(`Unstake should have failed before freeze period elapsed, but succeeded with tx: ${tx}`);
    } catch (err) {
      if (err instanceof anchor.AnchorError && err.error.errorCode.code === "FreezePeriodNotElapsed") {
        console.log("\nUnstake failed as expected:", err.error.errorMessage);
      } else {
        throw err;
      }
    }
  });

  it("Time travel to the future", async () => {
    // Advance time in milliseconds
    const currentTimestamp = Date.now();
    await advanceTime({ absoluteTimestamp: currentTimestamp + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY });
    console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS)
  });

  it("Claim rewards after time travel", async () => {
    const userRewardsAta = getAssociatedTokenAddressSync(rewardsMint, provider.wallet.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    const tx = await program.methods.claimRewards()
    .accountsPartial({
      owner: provider.wallet.publicKey,
      updateAuthority,
      config,
      rewardsMint,
      userRewardsAta,
      asset: nftKeypair.publicKey,
      collection: collectionKeypair.publicKey,
      mplCoreProgram: MPL_CORE_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .rpc();
    console.log("\nClaim rewards signature", tx);

    const balance = (await provider.connection.getTokenAccountBalance(userRewardsAta)).value.uiAmount;
    console.log("User rewards balance after claim:", balance);
    // 8 days * (10000 bps = 1.0) * (10^6 decimals / 10^6) = 8.0 tokens
    anchor.assert.ok(balance > 0);

    // Verify last_claimed_at is updated
    const assetInfo = await provider.connection.getAccountInfo(nftKeypair.publicKey);
    const stakedAtStr = getAttribute(assetInfo.data, "staked_at");
    const lastClaimedAtStr = getAttribute(assetInfo.data, "last_claimed_at");
    console.log("Staked at:", stakedAtStr, "Last claimed at:", lastClaimedAtStr);
    anchor.assert.ok(Number(lastClaimedAtStr) > Number(stakedAtStr));

    // Verify collection count is still 1
    const collectionInfo = await provider.connection.getAccountInfo(collectionKeypair.publicKey);
    const totalStakedStr = getAttribute(collectionInfo.data, "total_staked");
    anchor.assert.equal(totalStakedStr, "1");
  });

  it("Try to claim rewards again immediately", async () => {
    const userRewardsAta = getAssociatedTokenAddressSync(rewardsMint, provider.wallet.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    try {
      await program.methods.claimRewards()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        asset: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
      throw new Error("Claim rewards should have failed again immediately because 0 days elapsed since last claim");
    } catch (err) {
      if (err instanceof anchor.AnchorError && err.error.errorCode.code === "NoRewardsToClaim") {
        console.log("\nClaim rewards failed as expected:", err.error.errorMessage);
      } else {
        throw err;
      }
    }
  });

  it("Unstake an NFT and check remaining rewards and statistics", async () => {
    // Get the user rewards ATA account
    const userRewardsAta = getAssociatedTokenAddressSync(rewardsMint, provider.wallet.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
    const initialBalance = (await provider.connection.getTokenAccountBalance(userRewardsAta)).value.uiAmount;

    const tx = await program.methods.unstake()
    .accountsPartial({
      owner: provider.wallet.publicKey,
      updateAuthority,
      config,
      rewardsMint,
      userRewardsAta,
      asset: nftKeypair.publicKey,
      collection: collectionKeypair.publicKey,
      mplCoreProgram: MPL_CORE_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .rpc();
    console.log("\nUnstake transaction signature", tx);

    const finalBalance = (await provider.connection.getTokenAccountBalance(userRewardsAta)).value.uiAmount;
    console.log("User final rewards balance:", finalBalance);
    // Since we claim at 8 days and unstake at 8 days, we get 0 additional rewards
    anchor.assert.equal(finalBalance, initialBalance);

    // Verify statistics on collection is decremented to 0
    const collectionInfo = await provider.connection.getAccountInfo(collectionKeypair.publicKey);
    const totalStakedStr = getAttribute(collectionInfo.data, "total_staked");
    console.log("Collection total staked after unstake:", totalStakedStr);
    anchor.assert.equal(totalStakedStr, "0");

    // Verify asset is unstaked
    const assetInfo = await provider.connection.getAccountInfo(nftKeypair.publicKey);
    const stakedStr = getAttribute(assetInfo.data, "staked");
    anchor.assert.equal(stakedStr, "false");
  });
});
