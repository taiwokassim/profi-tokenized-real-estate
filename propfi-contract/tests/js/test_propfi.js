const anchor = require('@project-serum/anchor');
const { SystemProgram } = anchor.web3;

describe('propfi', () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Propfi;

  it('initializes a property', async () => {
    const owner = provider.wallet.publicKey;
    const [propertyPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from('property'), owner.toBuffer()],
      program.programId
    );

    await program.methods.initializeProperty(new anchor.BN(1000)).accounts({
      property: propertyPda,
      owner: owner,
      systemProgram: SystemProgram.programId,
    }).rpc();

    const prop = await program.account.property.fetch(propertyPda);
    console.log('property', prop);
  });
});
