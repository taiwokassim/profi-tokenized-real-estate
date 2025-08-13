import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Propfi } from "../target/types/propfi";
import { PublicKey, SystemProgram } from "@solana/web3.js";

describe("propfi", () => {
  // Set up the provider
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Propfi as Program<Propfi>;

  // PDA for property account
  let propertyAccountPda: PublicKey;

  it("Initialize a property", async () => {
    const propertyName = "Lekki Villa";
    const propertyValue = new anchor.BN(500000); // Example property value

    // Derive PDA for property account
    [propertyAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("property"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    // Call initialize_property
    const tx = await program.methods
      .initializeProperty(propertyName, propertyValue)
      .accounts({
        propertyAccount: propertyAccountPda,
        user: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Transaction signature:", tx);

    // Fetch the property account data
    const propertyAccount = await program.account.propertyAccount.fetch(propertyAccountPda);

    console.log("Property name:", propertyAccount.name);
    console.log("Property value:", propertyAccount.value.toString());

    // Assertions
    if (propertyAccount.name !== propertyName) {
      throw new Error("Property name does not match!");
    }
    if (!propertyAccount.value.eq(propertyValue)) {
      throw new Error("Property value does not match!");
    }
  });
});
