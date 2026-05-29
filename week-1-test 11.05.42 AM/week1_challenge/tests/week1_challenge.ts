import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Week1Challenge } from "../target/types/week1_challenge";

describe("week1_challenge", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.week1Challenge as Program<Week1Challenge>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.createVault().rpc();
    console.log("Your transaction signature", tx);
  });
});
