import * as anchor from "@coral-xyz/anchor"
import { Program } from "@coral-xyz/anchor"
import { Amm } from "../target/types/amm"
import { TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction, createMint, getAccount, getAssociatedTokenAddress, getMint, mintTo } from "@solana/spl-token"
import { assert } from "chai"

describe("amm initialize", () => {
    const provider = anchor.AnchorProvider.local()
    anchor.setProvider(provider)
    const program = anchor.workspace.Amm as Program<Amm>
    const payer = provider.wallet as anchor.Wallet
    let mintToken1: anchor.web3.PublicKey
    let mintToken2: anchor.web3.PublicKey
    let dataAccountPda: anchor.web3.PublicKey
    let dataAccountBump: number
    let lpMintPda: anchor.web3.PublicKey
    let lpMintBump: number
    let token1Ata: anchor.web3.PublicKey
    let token2Ata: anchor.web3.PublicKey
    let token1PoolAta: anchor.web3.PublicKey
    let token2PoolAta: anchor.web3.PublicKey
    const secondUser = anchor.web3.Keypair.generate()
    let secondUserToken1Ata: anchor.web3.PublicKey
    let secondUserToken2Ata: anchor.web3.PublicKey
    let connection = anchor.getProvider().connection

    before(async () => {
        const tx = await connection.requestAirdrop(secondUser.publicKey, 6 * 1000000000)
        await connection.confirmTransaction(tx)
        mintToken1 = await createMint(
            provider.connection,
            payer.payer,
            payer.publicKey,
            null,
            6
        )
        mintToken2 = await createMint(
            provider.connection,
            payer.payer,
            payer.publicKey,
            null,
            6
        );
        [dataAccountPda, dataAccountBump] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("dataAccount"), program.programId.toBuffer()],
            program.programId
        );
        [lpMintPda, lpMintBump] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("mint")],
            program.programId
        )
        const [poolAuthorityPda, poolAuthorityBump] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("pool_authority")],
            program.programId
        )
        token1Ata = await getAssociatedTokenAddress(
            mintToken1,
            poolAuthorityPda,
            true,
            TOKEN_PROGRAM_ID
        )
        token2Ata = await getAssociatedTokenAddress(
            mintToken2,
            poolAuthorityPda,
            true,
            TOKEN_PROGRAM_ID
        )
        token1PoolAta = await getAssociatedTokenAddress(
            mintToken1,
            poolAuthorityPda,
            true,
            TOKEN_PROGRAM_ID
        )
        token2PoolAta = await getAssociatedTokenAddress(
            mintToken2,
            poolAuthorityPda,
            true,
            TOKEN_PROGRAM_ID
        )
        secondUserToken1Ata = await getAssociatedTokenAddress(
            mintToken1,
            secondUser.publicKey,
            true,
            TOKEN_PROGRAM_ID
        )
        secondUserToken2Ata = await getAssociatedTokenAddress(
            mintToken2,
            secondUser.publicKey,
            true,
            TOKEN_PROGRAM_ID
        )
    })

    it("Initializes the AMM", async () => {
        await program.methods
            .initialize()
            .accounts({
                signer: payer.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                mintToken1: mintToken1,
                mintToken2: mintToken2,
            })
            .rpc()
        const dataAccount = await program.account.dataAccount.fetch(dataAccountPda)
        assert.ok(dataAccount.token1Mint.equals(mintToken1))
        assert.ok(dataAccount.token2Mint.equals(mintToken2))
        const ata1Info = await getAccount(provider.connection, token1Ata)
        const ata2Info = await getAccount(provider.connection, token2Ata)
        assert.strictEqual(Number(ata1Info.amount), 0, "Token1 ATA should have 0 balance")
        assert.strictEqual(Number(ata2Info.amount), 0, "Token2 ATA should have 0 balance")
    })

    it("Adds liquidity", async () => {
        const token1Amount = 1_000_000
        const token2Amount = 2_000_000
        const token1UserAta = await getAssociatedTokenAddress(
            mintToken1,
            payer.publicKey
        )
        const token2UserAta = await getAssociatedTokenAddress(
            mintToken2,
            payer.publicKey
        )
        const userLpAta = await getAssociatedTokenAddress(
            lpMintPda,
            payer.publicKey
        )
        const tx = new anchor.web3.Transaction()
        tx.add(
            createAssociatedTokenAccountInstruction(
                payer.publicKey,
                token1UserAta,
                payer.publicKey,
                mintToken1
            ),
            createAssociatedTokenAccountInstruction(
                payer.publicKey,
                token2UserAta,
                payer.publicKey,
                mintToken2
            ),
        )
        await provider.sendAndConfirm(tx)
        await mintTo(
            provider.connection,
            payer.payer,
            mintToken1,
            token1UserAta,
            payer.payer,
            token1Amount
        )
        await mintTo(
            provider.connection,
            payer.payer,
            mintToken2,
            token2UserAta,
            payer.payer,
            token2Amount
        )
        await program.methods
            .addLiquidity(new anchor.BN(token1Amount), new anchor.BN(token2Amount))
            .accounts({
                signer: payer.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                mintToken1: mintToken1,
                mintToken2: mintToken2,
            })
            .rpc()
        const updatedDataAccount = await program.account.dataAccount.fetch(dataAccountPda)
        assert.strictEqual(
            updatedDataAccount.token1Balance.toNumber(),
            token1Amount,
            "DataAccount token1 balance should update"
        )
        assert.strictEqual(
            updatedDataAccount.token2Balance.toNumber(),
            token2Amount,
            "DataAccount token2 balance should update"
        )
        const token1PoolAccountInfo = await getAccount(provider.connection, token1PoolAta)
        const token2PoolAccountInfo = await getAccount(provider.connection, token2PoolAta)
        assert.strictEqual(
            Number(token1PoolAccountInfo.amount),
            token1Amount,
            "Pool token1 account balance should be equal to token1Amount"
        )
        assert.strictEqual(
            Number(token2PoolAccountInfo.amount),
            token2Amount,
            "Pool token2 account balance should be equal to token2Amount"
        )
        let userLpAccountInfo = await getAccount(provider.connection, userLpAta)
        userLpAccountInfo = await getAccount(provider.connection, userLpAta)
        assert.ok(userLpAccountInfo.amount > 0, "User should receive LP tokens")
    })

    it("second user test", async () => {
        const token1Amount = 500_000
        const token2Amount = 1_000_000
        const token1UserAta = await getAssociatedTokenAddress(
            mintToken1,
            secondUser.publicKey
        )
        const token2UserAta = await getAssociatedTokenAddress(
            mintToken2,
            secondUser.publicKey
        )
        const userLpAta = await getAssociatedTokenAddress(
            lpMintPda,
            secondUser.publicKey
        )
        const tx = new anchor.web3.Transaction()
        tx.add(
            createAssociatedTokenAccountInstruction(
                secondUser.publicKey,
                token1UserAta,
                secondUser.publicKey,
                mintToken1
            ),
            createAssociatedTokenAccountInstruction(
                secondUser.publicKey,
                token2UserAta,
                secondUser.publicKey,
                mintToken2
            ),
        )
        await provider.sendAndConfirm(tx, [secondUser])
        await mintTo(
            provider.connection,
            payer.payer,
            mintToken1,
            secondUserToken1Ata,
            payer.publicKey,
            token1Amount,
        )
        await mintTo(
            provider.connection,
            payer.payer,
            mintToken2,
            secondUserToken2Ata,
            payer.publicKey,
            token2Amount,
        )
        await program.methods
            .addLiquidity(new anchor.BN(token1Amount), new anchor.BN(token2Amount))
            .accounts({
                signer: secondUser.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                mintToken1: mintToken1,
                mintToken2: mintToken2,
            })
            .signers([secondUser])
            .rpc()
        const updatedDataAccount = await program.account.dataAccount.fetch(dataAccountPda)
        assert.strictEqual(
            updatedDataAccount.token1Balance.toNumber(),
            token1Amount + 1_000_000,
            "DataAccount token1 balance should update"
        )
        assert.strictEqual(
            updatedDataAccount.token2Balance.toNumber(),
            token2Amount + 2_000_000,
            "DataAccount token2 balance should update"
        )
        const token1PoolAccountInfo = await getAccount(provider.connection, token1PoolAta)
        const token2PoolAccountInfo = await getAccount(provider.connection, token2PoolAta)
        assert.strictEqual(
            Number(token1PoolAccountInfo.amount),
            token1Amount + 1_000_000,
            "Pool token1 account balance should be equal to token1Amount"
        )
        assert.strictEqual(
            Number(token2PoolAccountInfo.amount),
            token2Amount + 2_000_000,
            "Pool token2 account balance should be equal to token2Amount"
        )
        const userLpAta1 = await getAssociatedTokenAddress(
            lpMintPda,
            payer.publicKey
        )
        let userLpAccountInfo1 = await getAccount(provider.connection, userLpAta1)
        userLpAccountInfo1 = await getAccount(provider.connection, userLpAta1)
        let userLpAccountInfo2 = await getAccount(provider.connection, userLpAta)
        userLpAccountInfo2 = await getAccount(provider.connection, userLpAta)
        assert.ok(userLpAccountInfo2.amount > 0, "User should receive LP tokens")
        assert.ok(Number(userLpAccountInfo2.amount) == Math.floor(Number(userLpAccountInfo1.amount) / 2), "Varying data")
    })

    it("Quotes output amount correctly", async () => {
        const dataAccount = await program.account.dataAccount.fetch(dataAccountPda)
        const token1Balance = dataAccount.token1Balance.toNumber()
        const token2Balance = dataAccount.token2Balance.toNumber()
        const token1Mint = dataAccount.token1Mint
        const amountToQuote = 100_000
        const quotedAmount = await program.methods
            .quote(token1Mint, new anchor.BN(amountToQuote))
            .accounts({
                dataAccount: dataAccountPda,
            })
            .view()
        const feeNumerator = BigInt(3)
        const feeDenominator = BigInt(1000)
        const feeAmount = BigInt(amountToQuote) * feeNumerator / feeDenominator
        const amountAfterFee = BigInt(amountToQuote) - feeAmount
        const k = BigInt(token1Balance) * BigInt(token2Balance)
        const newT1Balance = BigInt(token1Balance) + amountAfterFee
        const res = k / newT1Balance
        const expected = BigInt(token2Balance) - res
        assert.strictEqual(quotedAmount.toString(), expected.toString())
    })

    it("Swaps token1 for token2 with full balance checks", async () => {
        const swapAmount = 100_000
        await mintTo(
            connection,
            payer.payer,
            mintToken1,
            secondUserToken1Ata,
            payer.publicKey,
            swapAmount
        )
        const [poolAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("pool_authority")],
            program.programId
        )
        const poolToken1Ata = await getAssociatedTokenAddress(mintToken1, poolAuthorityPda, true)
        const poolToken2Ata = await getAssociatedTokenAddress(mintToken2, poolAuthorityPda, true)
        const userToken1Before = Number((await getAccount(connection, secondUserToken1Ata)).amount)
        const userToken2Before = Number((await getAccount(connection, secondUserToken2Ata)).amount)
        const poolToken1Before = Number((await getAccount(connection, poolToken1Ata)).amount)
        const poolToken2Before = Number((await getAccount(connection, poolToken2Ata)).amount)
        let dataAcc = await program.account.dataAccount.fetch(dataAccountPda)
        const quotedAmount = await program.methods
            .quote(mintToken1, new anchor.BN(swapAmount))
            .accounts({ dataAccount: dataAccountPda })
            .view()
        await program.methods
            .swap(new anchor.BN(swapAmount), mintToken1)
            .accounts({
                signer: secondUser.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                mintToken1,
                mintToken2,
            })
            .signers([secondUser])
            .rpc()
        const userToken1After = Number((await getAccount(connection, secondUserToken1Ata)).amount)
        const userToken2After = Number((await getAccount(connection, secondUserToken2Ata)).amount)
        const poolToken1After = Number((await getAccount(connection, poolToken1Ata)).amount)
        const poolToken2After = Number((await getAccount(connection, poolToken2Ata)).amount)
        dataAcc = await program.account.dataAccount.fetch(dataAccountPda)
        assert.strictEqual(userToken1After, userToken1Before - swapAmount, "User token1 should decrease by swap amount")
        assert.strictEqual(poolToken1After, poolToken1Before + swapAmount, "Pool token1 should increase by swap amount")
        assert.strictEqual(userToken2After, userToken2Before + Number(quotedAmount), "User token2 should increase by quoted amount")
        assert.strictEqual(poolToken2After, poolToken2Before - Number(quotedAmount), "Pool token2 should decrease by quoted amount")
    })

    it("Swaps token2 for token1 with full balance checks", async () => {
        const swapAmount = 100_000
        await mintTo(
            connection,
            payer.payer,
            mintToken2,
            secondUserToken2Ata,
            payer.publicKey,
            swapAmount
        )
        const [poolAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("pool_authority")],
            program.programId
        )
        const poolToken1Ata = await getAssociatedTokenAddress(mintToken1, poolAuthorityPda, true)
        const poolToken2Ata = await getAssociatedTokenAddress(mintToken2, poolAuthorityPda, true)
        const userToken1Before = Number((await getAccount(connection, secondUserToken1Ata)).amount)
        const userToken2Before = Number((await getAccount(connection, secondUserToken2Ata)).amount)
        const poolToken1Before = Number((await getAccount(connection, poolToken1Ata)).amount)
        const poolToken2Before = Number((await getAccount(connection, poolToken2Ata)).amount)
        let dataAcc = await program.account.dataAccount.fetch(dataAccountPda)
        const quotedAmount = await program.methods
            .quote(mintToken2, new anchor.BN(swapAmount))
            .accounts({ dataAccount: dataAccountPda })
            .view()
        await program.methods
            .swap(new anchor.BN(swapAmount), mintToken2)
            .accounts({
                signer: secondUser.publicKey,
                tokenProgram: TOKEN_PROGRAM_ID,
                mintToken1,
                mintToken2,
            })
            .signers([secondUser])
            .rpc()
        const userToken1After = Number((await getAccount(connection, secondUserToken1Ata)).amount)
        const userToken2After = Number((await getAccount(connection, secondUserToken2Ata)).amount)
        const poolToken1After = Number((await getAccount(connection, poolToken1Ata)).amount)
        const poolToken2After = Number((await getAccount(connection, poolToken2Ata)).amount)
        dataAcc = await program.account.dataAccount.fetch(dataAccountPda)
        assert.strictEqual(userToken2After, userToken2Before - swapAmount, "User token2 should decrease by swap amount")
        assert.strictEqual(poolToken2After, poolToken2Before + swapAmount, "Pool token2 should increase by swap amount")
        assert.strictEqual(userToken1After, userToken1Before + Number(quotedAmount), "User token1 should increase by quoted amount")
        assert.strictEqual(poolToken1After, poolToken1Before - Number(quotedAmount), "Pool token1 should decrease by quoted amount")
    })

    it("Burns LP tokens and withdraws liquidity correctly for second user", async () => {
        const token1UserAta = await getAssociatedTokenAddress(mintToken1, payer.publicKey);
        const token2UserAta = await getAssociatedTokenAddress(mintToken2, payer.publicKey);
        const userLpAta = await getAssociatedTokenAddress(lpMintPda, payer.publicKey);
        const [poolAuthorityPda] = await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("pool_authority")],
            program.programId
        );

        const poolToken1Ata = await getAssociatedTokenAddress(mintToken1, poolAuthorityPda, true);
        const poolToken2Ata = await getAssociatedTokenAddress(mintToken2, poolAuthorityPda, true);

        // Fetch balances before burning
        const userToken1Before = Number((await getAccount(connection, token1UserAta)).amount);
        const userToken2Before = Number((await getAccount(connection, token2UserAta)).amount);
        const poolToken1Before = Number((await getAccount(connection, poolToken1Ata)).amount);
        const poolToken2Before = Number((await getAccount(connection, poolToken2Ata)).amount);
        const userLpBefore = Number((await getAccount(connection, userLpAta)).amount);
        const totalLpBefore = Number((await getMint(connection, lpMintPda)).supply);

        // Assert user has LP tokens to burn
        assert(userLpBefore > 0, "User should have LP tokens before burning");

        // Call removeLiquidity with user's full LP balance
        await program.methods.removeLiquidity(new anchor.BN(userLpBefore)).accounts({
            signer: payer.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            mintToken1: mintToken1,
            mintToken2: mintToken2,
        }).rpc();

        // Fetch balances after burning
        const userToken1After = Number((await getAccount(connection, token1UserAta)).amount);
        const userToken2After = Number((await getAccount(connection, token2UserAta)).amount);
        const poolToken1After = Number((await getAccount(connection, poolToken1Ata)).amount);
        const poolToken2After = Number((await getAccount(connection, poolToken2Ata)).amount);
        const userLpAfter = Number((await getAccount(connection, userLpAta)).amount);
        const totalLpAfter = Number((await getMint(connection, lpMintPda)).supply);

        // Assert user LP tokens burned fully
        assert.strictEqual(userLpAfter, 0, "User LP tokens should be zero after burning");

        // Assert total LP supply decreased correctly (within small tolerance)
        const expectedTotalLpAfter = totalLpBefore - userLpBefore;
        assert.ok(
            Math.abs(totalLpAfter - expectedTotalLpAfter) <= 5,
            `Total LP supply after burning should decrease by burned amount. Expected ~${expectedTotalLpAfter}, got ${totalLpAfter}`
        );

        // Calculate expected token withdrawals proportionally
        const burnRatio = userLpBefore / totalLpBefore;
        const expectedUserToken1Increase = Math.floor(poolToken1Before * burnRatio);
        const expectedUserToken2Increase = Math.floor(poolToken2Before * burnRatio);

        // Check user token balances increased as expected (allow small tolerance)
        assert.ok(
            Math.abs(userToken1After - userToken1Before - expectedUserToken1Increase) <= 5,
            `User token1 balance should increase by ~${expectedUserToken1Increase}`
        );
        assert.ok(
            Math.abs(userToken2After - userToken2Before - expectedUserToken2Increase) <= 5,
            `User token2 balance should increase by ~${expectedUserToken2Increase}`
        );

        // Check pool token balances decreased as expected (allow small tolerance)
        assert.ok(
            Math.abs(poolToken1Before - poolToken1After - expectedUserToken1Increase) <= 5,
            `Pool token1 balance should decrease by ~${expectedUserToken1Increase}`
        );
        assert.ok(
            Math.abs(poolToken2Before - poolToken2After - expectedUserToken2Increase) <= 5,
            `Pool token2 balance should decrease by ~${expectedUserToken2Increase}`
        );
    });
})

