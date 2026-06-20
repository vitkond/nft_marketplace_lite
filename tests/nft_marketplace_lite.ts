import * as anchor from "@coral-xyz/anchor";
import {Program} from "@coral-xyz/anchor";
import {NftMarketplaceLite} from "../target/types/nft_marketplace_lite";
import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint, getAccount,
    getAssociatedTokenAddressSync,
    getOrCreateAssociatedTokenAccount,
    mintTo,
    TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import {expect} from "chai";

describe("nft_marketplace_lite", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.nftMarketplaceLite as Program<NftMarketplaceLite>;

    const price = new anchor.BN(2_000_000);

    it("cancel listing!", async () => {
        const seller = anchor.web3.Keypair.generate();
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                seller.publicKey,
                2 * anchor.web3.LAMPORTS_PER_SOL
            )
        );

        const mint = await createMint(
            provider.connection,
            provider.wallet.payer,
            provider.wallet.publicKey,
            null,
            0
        );

        const sellerAta = await getOrCreateAssociatedTokenAccount(
            provider.connection, provider.wallet.payer, mint, seller.publicKey
        );
        await mintTo(
            provider.connection,
            provider.wallet.payer,
            mint,
            sellerAta.address,
            provider.wallet.publicKey,
            1
        );


        const [listingPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("listing"), seller.publicKey.toBuffer(), mint.toBuffer()],
            program.programId
        );

        const vault = getAssociatedTokenAddressSync(
            mint, listingPda, true
        );


        await program.methods
            .listNft(price)
            .accountsPartial({
                seller: seller.publicKey,
                sellerNftAccount: sellerAta.address,
                mint,
                listing: listingPda,
                vault,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([seller])
            .rpc();

        const listing = await program.account.listing.fetch(listingPda);
        console.log(listing);

        expect(listing.mint.toBase58()).to.eq(
            mint.toBase58()
        );

        expect(listing.seller.toBase58()).to.eq(seller.publicKey.toBase58());

        const vaultAccount = await getAccount(provider.connection, vault);

        expect(vaultAccount.amount.toString()).to.eq("1");
        expect(vaultAccount.mint.toBase58()).to.eq(mint.toBase58());
        expect(vaultAccount.owner.toBase58()).to.eq(listingPda.toBase58());

        console.log("cancelling listing!");

        await program.methods
            .cancelListing()
            .accountsPartial({
                seller: seller.publicKey,
                sellerNftAccount: sellerAta.address,
                mint,
                listing: listingPda,
                vault,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .signers([seller])
            .rpc();

        const listingAccount = await provider.connection.getAccountInfo(listingPda);
        expect(listingAccount).to.eq(null);
        const sellerAfter = await getAccount(provider.connection, sellerAta.address);
        expect(sellerAfter.amount.toString()).to.eq("1");

    });

    it("buy nft!", async () => {
        const seller = anchor.web3.Keypair.generate();
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                seller.publicKey,
                2 * anchor.web3.LAMPORTS_PER_SOL
            )
        );

        const buyer = anchor.web3.Keypair.generate();
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                buyer.publicKey,
                2 * anchor.web3.LAMPORTS_PER_SOL
            )
        );

        const mint = await createMint(
            provider.connection,
            provider.wallet.payer,
            provider.wallet.publicKey,
            null,
            0
        );

        const sellerAta = await getOrCreateAssociatedTokenAccount(
            provider.connection, provider.wallet.payer, mint, seller.publicKey
        );
        await mintTo(
            provider.connection,
            provider.wallet.payer,
            mint,
            sellerAta.address,
            provider.wallet.publicKey,
            1
        );

        const bayerAta = await getOrCreateAssociatedTokenAccount(
            provider.connection, provider.wallet.payer, mint, buyer.publicKey
        );


        const [listingPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("listing"), seller.publicKey.toBuffer(), mint.toBuffer()],
            program.programId
        );

        const vault = getAssociatedTokenAddressSync(
            mint, listingPda, true
        );


        await program.methods
            .listNft(price)
            .accountsPartial({
                seller: seller.publicKey,
                sellerNftAccount: sellerAta.address,
                mint,
                listing: listingPda,
                vault,
                tokenProgram: TOKEN_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([seller])
            .rpc();

        await program.methods
            .buyNft(price)
            .accountsPartial({
                seller: seller.publicKey,
                mint,
                listing: listingPda,
                vault,
                tokenProgram: TOKEN_PROGRAM_ID,
                buyer: buyer.publicKey,
                buyerNftAccount: bayerAta.address,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([buyer])
            .rpc();

        const listingAccount = await provider.connection.getAccountInfo(listingPda);
        expect(listingAccount).to.eq(null);
        const buyerAfter = await getAccount(provider.connection, bayerAta.address);
        expect(buyerAfter.amount.toString()).to.eq("1");
    });
});
