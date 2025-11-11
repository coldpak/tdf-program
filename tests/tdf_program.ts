import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { TdfProgram } from "../target/types/tdf_program";

import { sendAndConfirmTransaction, Transaction } from "@solana/web3.js";
import {
  createMint,
  getAssociatedTokenAddressSync,
  getMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { getAuthToken } from "./tee-getAuthToken";

const RPC_URL = "https://api.devnet.solana.com";
const WS_URL = "wss://api.devnet.solana.com";
const MAGICBLOCK_RPC_URL = "https://devnet.magicblock.app";
const MAGICBLOCK_WS_URL = "wss://devnet.magicblock.app";
const TEE_RPC_URL = "https://tee.magicblock.app/";
const TEE_WS_URL = "wss://tee.magicblock.app/";

const SEED_GLOBAL_CONFIG = "global_config";
const PYTH_LAZER_ID = 6; // SOLUSD
const PYTH_EXPONENT = -8;
const PYTH_SEED = [
  Buffer.from("price_feed"),
  Buffer.from("pyth-lazer"),
  Buffer.from(PYTH_LAZER_ID.toString()),
];
const SOL_DECIMALS = 6;
const PYTH_PROGRAM_ID = "PriCems5tHihc6UDXDjzjeawomAwBduWMGAi8ZUjppd";
const [price_feed_pda] = anchor.web3.PublicKey.findProgramAddressSync(
  PYTH_SEED,
  new anchor.web3.PublicKey(PYTH_PROGRAM_ID)
);

const ENTRY_TOKEN_MINT = "6kyfSqqp9xE4etzxeKibDfSW1rmAtCdZDTndEKoCxTfw";
const ENTRY_TOKEN_DECIMALS = 6;
const ENTRY_AMOUNT = 1000000000; // 1,000 entry token
const VIRTUAL_ON_DEPOSIT = 10000000000; // $10,000 paper dollars

const LEAGUE_ID = "1234567890";

describe("tdf-program", async () => {
  const { getDelegationStatus, getClosestValidator, sendMagicTransaction } =
    await require("magic-router-sdk");
  const { groupPdaFromId, PERMISSION_PROGRAM_ID, permissionPdaFromAccount } =
    await require("@magicblock-labs/ephemeral-rollups-sdk/privacy");

  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.tdfProgram as Program<TdfProgram>;

  const connection = new web3.Connection(RPC_URL, {
    wsEndpoint: WS_URL,
  });

  const mbConnection = new web3.Connection(MAGICBLOCK_RPC_URL, {
    wsEndpoint: MAGICBLOCK_WS_URL,
  });

  const routerConnection = new web3.Connection(
    process.env.ROUTER_ENDPOINT || "https://devnet-router.magicblock.app",
    {
      wsEndpoint:
        process.env.ROUTER_WS_ENDPOINT || "wss://devnet-router.magicblock.app",
    }
  );

  const teeConnection = new web3.Connection(TEE_RPC_URL, {
    wsEndpoint: TEE_WS_URL,
  });

  const [global_config_pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(SEED_GLOBAL_CONFIG)],
    program.programId
  );

  console.log("Router Endpoint: ", routerConnection.rpcEndpoint);
  console.log("Program ID: ", program.programId.toBase58());

  const [marketPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("market"), price_feed_pda.toBuffer()],
    program.programId
  );
  const entryTokenMintPda = new anchor.web3.PublicKey(ENTRY_TOKEN_MINT);

  const [leaguePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("league"),
      anchor.Wallet.local().publicKey.toBuffer(),
      Buffer.from(LEAGUE_ID),
    ],
    program.programId
  );

  const [leaderboardPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("leaderboard"), leaguePda.toBuffer()],
    program.programId
  );

  const rewardVaultPda = getAssociatedTokenAddressSync(
    entryTokenMintPda,
    leaguePda,
    true
  );

  const [participantPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("participant"),
      leaguePda.toBuffer(),
      anchor.Wallet.local().publicKey.toBuffer(),
    ],
    program.programId
  );
  const currentPositionSeq = 1; // TODO: get from participant
  // Convert to little-endian bytes (u64 = 8 bytes)
  const seqBytes = Buffer.allocUnsafe(8);
  seqBytes.writeBigUInt64LE(BigInt(currentPositionSeq.toString()), 0);
  const [positionPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("position"),
      leaguePda.toBuffer(),
      anchor.Wallet.local().publicKey.toBuffer(),
      seqBytes,
    ],
    program.programId
  );

  const privatePositionSeq = 2;
  const seqBytesPrivate = Buffer.allocUnsafe(8);
  seqBytesPrivate.writeBigUInt64LE(BigInt(privatePositionSeq.toString()), 0);
  const [privatePositionPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("position"),
      leaguePda.toBuffer(),
      anchor.Wallet.local().publicKey.toBuffer(),
      seqBytesPrivate,
    ],
    program.programId
  );

  // Listup known pdas
  console.log("Global Config PDA: ", global_config_pda.toBase58());
  console.log("Price Feed PDA: ", price_feed_pda.toBase58());
  console.log("Market PDA: ", marketPda.toBase58());
  console.log("Entry Token Mint PDA: ", entryTokenMintPda.toBase58());
  console.log("League PDA: ", leaguePda.toBase58());
  console.log("Leaderboard PDA: ", leaderboardPda.toBase58());
  console.log("Reward Vault PDA: ", rewardVaultPda.toBase58());
  console.log("Participant PDA: ", participantPda.toBase58());
  console.log("Position PDA: ", positionPda.toBase58());

  // Permission TEE AuthToken
  let authTokenAdmin: string = "";
  let providerTeeAdmin: anchor.AnchorProvider;

  it("Initialize GlobalConfig with 10% fee, only if it doesn't exist", async () => {
    // Check if GlobalConfig exists
    const existingConfig = await program.account.globalConfig.fetchNullable(
      global_config_pda
    );

    if (!existingConfig) {
      const tx = (await program.methods
        .initialize(1000)
        .accounts({
          // @ts-ignore
          globalConfig: global_config_pda,
          admin: anchor.Wallet.local().publicKey,
          treasury: anchor.Wallet.local().publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .transaction()) as Transaction;
      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);
      console.log("✅ Initialized Global Config! Signature:", signature);
    } else {
      console.log("ℹ️ Global Config already initialized");
    }
  });

  it("Check Price Feed", async () => {
    const pythAccountInfo = await mbConnection.getAccountInfo(price_feed_pda);
    const PRICE_OFFSET = 73;

    const dataView = new DataView(
      pythAccountInfo.data.buffer,
      pythAccountInfo.data.byteOffset,
      pythAccountInfo.data.length
    );
    const priceInt =
      Number(dataView.getBigUint64(PRICE_OFFSET, true)) *
      Math.pow(10, PYTH_EXPONENT);
    console.log("SOL price:", priceInt);
  });

  it("Create Market", async () => {
    // Check if market exists
    const existingMarket = await program.account.market.fetchNullable(
      marketPda
    );

    if (!existingMarket) {
      console.log("Creating Market...");
      const tx = await program.methods
        .createMarket(
          // @ts-ignore
          Buffer.from("SOLUSD"),
          SOL_DECIMALS,
          20
        )
        .accounts({
          // @ts-ignore
          market: marketPda,
          priceFeed: price_feed_pda,
          globalConfig: global_config_pda,
          admin: anchor.Wallet.local().publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: anchor.Wallet.local().publicKey,
        })
        .transaction();

      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);

      console.log("✅ Created Market! Signature:", signature);
    } else {
      console.log("ℹ️ Market already exists");
    }
  });

  it("Update Market", async () => {
    const tx = await program.methods
      .updateMarket(
        // @ts-ignore
        Buffer.from("SOLUSD"),
        SOL_DECIMALS,
        true,
        20
      )
      .accounts({
        // @ts-ignore
        market: marketPda,
        priceFeed: price_feed_pda,
        globalConfig: global_config_pda,
        admin: anchor.Wallet.local().publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .transaction();

    const signature = await sendMagicTransaction(routerConnection, tx, [
      anchor.Wallet.local().payer,
    ]);

    console.log("✅ Updated Market! Signature:", signature);
  });

  it("Create Entry Token Mint", async () => {
    try {
      const tokenExist = await connection.getAccountInfo(
        new anchor.web3.PublicKey(ENTRY_TOKEN_MINT)
      );
      if (tokenExist) {
        console.log("ℹ️ Entry Token Mint already exists: ", ENTRY_TOKEN_MINT);
      } else {
        // Create a new token
        const entryTokenMint = await createMint(
          connection,
          anchor.Wallet.local().payer,
          anchor.Wallet.local().publicKey,
          null,
          ENTRY_TOKEN_DECIMALS
        );

        console.log(
          "✅ Created Entry Token Mint! Mint Address:",
          entryTokenMint.toString()
        );
      }
    } catch (error) {
      console.error("❌ Error creating entry token mint:", error);
    }
  });

  it("Mint Entry Token if not enough", async () => {
    try {
      const tokenAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        anchor.Wallet.local().payer,
        new anchor.web3.PublicKey(ENTRY_TOKEN_MINT),
        anchor.Wallet.local().publicKey
      );

      console.log("Token Account: ", tokenAccount.address);
      console.log("Amount: ", tokenAccount.amount);

      if (tokenAccount.amount < ENTRY_AMOUNT) {
        console.log("Minting Entry Token...");

        await mintTo(
          connection,
          anchor.Wallet.local().payer,
          new anchor.web3.PublicKey(ENTRY_TOKEN_MINT),
          tokenAccount.address,
          anchor.Wallet.local().payer,
          ENTRY_AMOUNT
        );
      }
    } catch (error) {
      console.error("❌ Error minting entry token:", error);
    }
  });

  // League
  it("Create League", async () => {
    // Check if league exists
    const existingLeague = await program.account.league.fetchNullable(
      leaguePda
    );

    const startTime = Math.floor(new Date().getTime() / 1000);
    const endTime = startTime + 86400; // 1 day

    if (!existingLeague) {
      console.log("Creating League...");
      const tx = await program.methods
        .createLeague(
          // @ts-ignore
          LEAGUE_ID,
          [marketPda],
          new anchor.BN(ENTRY_AMOUNT),
          new anchor.BN(VIRTUAL_ON_DEPOSIT),
          new anchor.BN(startTime),
          new anchor.BN(endTime),
          "https://example.com",
          100,
          20,
          5
        )
        .accounts({
          // @ts-ignore
          creator: anchor.Wallet.local().publicKey,
          // @ts-ignore
          league: leaguePda,
          leaderboard: leaderboardPda,
          entryTokenMint: entryTokenMintPda,
          rewardVault: rewardVaultPda,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: new anchor.web3.PublicKey(TOKEN_PROGRAM_ID),
          associatedTokenProgram: new anchor.web3.PublicKey(
            ASSOCIATED_TOKEN_PROGRAM_ID
          ),
        })
        .transaction();

      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);

      console.log("✅ Created League! Signature:", signature);
    } else {
      console.log("ℹ️ League already exists");
    }
  });

  it("Start League", async () => {
    // Check league status
    const leagueAccount = await program.account.league.fetch(leaguePda);
    if (leagueAccount.status.toString() === "pending") {
      const tx = await program.methods
        .startLeague()
        .accounts({
          league: leaguePda,
          user: anchor.Wallet.local().publicKey,
        })
        .transaction();

      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);
      console.log("✅ Started League! Signature:", signature);
    } else {
      console.log("ℹ️ League is not pending, skipping start league");
    }
  });

  it("Join League", async () => {
    // Check if participant exists
    const participantAccount = await program.account.participant.fetchNullable(
      participantPda
    );
    if (participantAccount) {
      console.log("ℹ️ Participant already exists, skipping join league");
    } else {
      console.log("Joining League...");

      const userEntryTokenAccountPda = getAssociatedTokenAddressSync(
        entryTokenMintPda,
        anchor.Wallet.local().publicKey,
        true
      );

      const tx = await program.methods
        .joinLeague()
        .accounts({
          league: leaguePda,
          // @ts-ignore
          participant: participantPda,
          rewardVault: rewardVaultPda,
          userEntryTokenAccount: userEntryTokenAccountPda,
          user: anchor.Wallet.local().publicKey,
          tokenProgram: new anchor.web3.PublicKey(TOKEN_PROGRAM_ID),
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .transaction();

      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);

      console.log("✅ Joined League! Signature:", signature);
    }
  });

  it("Delegate Participant", async () => {
    const delegated = await getDelegationStatus(
      routerConnection,
      participantPda
    );
    if (delegated.isDelegated) {
      console.log("ℹ️ Participant already delegated");
    } else {
      const tx = await program.methods
        .delegateParticipant(leaguePda)
        .accounts({
          user: anchor.Wallet.local().publicKey,
          participant: participantPda,
        })
        .transaction();

      const signature = await sendMagicTransaction(routerConnection, tx, [
        anchor.Wallet.local().payer,
      ]);

      console.log("✅ Delegated Participant! Signature:", signature);

      await sleepWithAnimation(5);
    }
  });

  // describe("Open Public Position Scenario", () => {
  //   it("Init Unopened Position and Delegate", async () => {
  //     // Check if position exists
  //     const positionAccount = await program.account.position.fetchNullable(
  //       positionPda
  //     );
  //     if (positionAccount) {
  //       console.log(
  //         "ℹ️ Position already exists, skipping init unopened position"
  //       );
  //     } else {
  //       console.log("Init Unopened Position...");
  //       const tx = await program.methods
  //         .initUnopenedPosition(
  //           leaguePda,
  //           new anchor.BN(currentPositionSeq),
  //           marketPda,
  //           SOL_DECIMALS
  //         )
  //         .accounts({
  //           // @ts-ignore
  //           position: positionPda,
  //           priceFeed: price_feed_pda,
  //           user: anchor.Wallet.local().publicKey,
  //           systemProgram: anchor.web3.SystemProgram.programId,
  //         })
  //         .transaction();

  //       // Send transaction to Base Layer
  //       const signature = await sendAndConfirmTransaction(connection, tx, [
  //         anchor.Wallet.local().payer,
  //       ]);

  //       console.log(
  //         "✅ Init Unopened Position and Delegated! Signature:",
  //         signature
  //       );
  //       await sleepWithAnimation(5);
  //     }

  //     const isPositionDelegated = await getDelegationStatus(
  //       routerConnection,
  //       positionPda
  //     );
  //     if (isPositionDelegated.isDelegated) {
  //       console.log(
  //         "ℹ️ Position already delegated, skipping delegate unopened position"
  //       );
  //     } else {
  //       console.log("Delegating Unopened Position...");

  //       const delegateTx = await program.methods
  //         .delegateUnopenedPosition(
  //           participantPda,
  //           new anchor.BN(currentPositionSeq)
  //         )
  //         .accounts({
  //           user: anchor.Wallet.local().publicKey,
  //           league: leaguePda,
  //           position: positionPda,
  //         })
  //         .transaction();

  //       const delegateSignature = await sendAndConfirmTransaction(
  //         connection,
  //         delegateTx,
  //         [anchor.Wallet.local().payer]
  //       );

  //       console.log(
  //         "✅ Delegated Unopened Position! Signature:",
  //         delegateSignature
  //       );

  //       await sleepWithAnimation(5);
  //     }
  //   });

  //   it("Open Position", async () => {
  //     const positionAccount = await mbConnection.getAccountInfo(positionPda);

  //     // Check if position's openedAt is 0
  //     const positionOffset = 187;

  //     const dataView = new DataView(
  //       positionAccount.data.buffer,
  //       positionAccount.data.byteOffset,
  //       positionAccount.data.length
  //     );

  //     const openedAt = dataView.getBigInt64(positionOffset, true);

  //     if (openedAt > 0) {
  //       console.log("ℹ️ Position already opened, skipping open position");
  //     } else {
  //       const tx = await program.methods
  //         .openPosition(
  //           new anchor.BN(currentPositionSeq),
  //           { long: {} },
  //           new anchor.BN(1000000), // 1 SOL
  //           1
  //         )
  //         .accounts({
  //           user: anchor.Wallet.local().publicKey,
  //           // @ts-ignore
  //           position: positionPda,
  //           priceFeed: price_feed_pda,
  //           league: leaguePda,
  //           market: marketPda,
  //           participant: participantPda,
  //         })
  //         .transaction();

  //       const signature = await sendMagicTransaction(routerConnection, tx, [
  //         anchor.Wallet.local().payer,
  //       ]);

  //       console.log("✅ Opened Position! Signature:", signature);
  //     }
  //   });

  //   it("Update Participant", async () => {
  //     const tx = await program.methods
  //       .updateParticipant(leaguePda, anchor.Wallet.local().publicKey)
  //       .accounts({
  //         // @ts-ignore
  //         participant: participantPda,
  //         leaderboard: leaderboardPda,
  //         payer: anchor.Wallet.local().publicKey,
  //         programId: program.programId,
  //       })
  //       .remainingAccounts([
  //         {
  //           pubkey: positionPda,
  //           isWritable: true,
  //           isSigner: false,
  //         },
  //         {
  //           pubkey: price_feed_pda,
  //           isWritable: false,
  //           isSigner: false,
  //         },
  //       ])
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Updated Participant! Signature:", signature);
  //   });

  //   it("Update and Commit Participant", async () => {
  //     const tx = await program.methods
  //       .updateAndCommitParticipant(leaguePda, anchor.Wallet.local().publicKey)
  //       .accounts({
  //         // @ts-ignore
  //         participant: participantPda,
  //         leaderboard: leaderboardPda,
  //         payer: anchor.Wallet.local().publicKey,
  //         programId: program.programId,
  //       })
  //       .remainingAccounts([
  //         {
  //           pubkey: positionPda,
  //           isWritable: true,
  //           isSigner: false,
  //         },
  //         {
  //           pubkey: price_feed_pda,
  //           isWritable: false,
  //           isSigner: false,
  //         },
  //       ])
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Updated and Committed Participant! Signature:", signature);
  //     await sleepWithAnimation(5);
  //   });

  //   it("Update Leaderboard", async () => {
  //     const tx = await program.methods
  //       .updateLeaderboardWithParticipant()
  //       .accounts({
  //         // @ts-ignore
  //         leaderboard: leaderboardPda,
  //         league: leaguePda,
  //         participant: participantPda,
  //       })
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Updated Leaderboard! Signature:", signature);
  //   });

  //   it("Close Position = Commit and Undelegate Position, Commit Participant", async () => {
  //     const tx = await program.methods
  //       .closePosition(new anchor.BN(currentPositionSeq))
  //       .accounts({
  //         user: anchor.Wallet.local().publicKey,
  //         // @ts-ignore
  //         position: positionPda,
  //         participant: participantPda,
  //         league: leaguePda,
  //         market: marketPda,
  //         priceFeed: price_feed_pda,
  //       })
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Closed Position! Signature:", signature);
  //     await sleepWithAnimation(5);
  //   });

  //   it("Commit Position and Commit Participant", async () => {
  //     const tx = await program.methods
  //       .commitPosition(
  //         leaguePda,
  //         anchor.Wallet.local().publicKey,
  //         new anchor.BN(currentPositionSeq)
  //       )
  //       .accounts({
  //         payer: anchor.Wallet.local().publicKey,
  //         // @ts-ignore
  //         position: positionPda,
  //       })
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Committed Position! Signature:", signature);

  //     const commitParticipantTx = await program.methods
  //       .commitParticipant(leaguePda, anchor.Wallet.local().publicKey)
  //       .accounts({
  //         payer: anchor.Wallet.local().publicKey,
  //         // @ts-ignore
  //         participant: participantPda,
  //         leaderboard: leaderboardPda,
  //         programId: program.programId,
  //       })
  //       .transaction();

  //     const commitParticipantSignature = await sendMagicTransaction(
  //       routerConnection,
  //       commitParticipantTx,
  //       [anchor.Wallet.local().payer]
  //     );

  //     console.log(
  //       "✅ Committed Participant! Signature:",
  //       commitParticipantSignature
  //     );

  //     await sleepWithAnimation(5);
  //   });

  //   it("Check Participant, Position, Leaderboard", async () => {
  //     const positionAtMB = await mbConnection.getAccountInfo(positionPda);
  //     const positionAtMBData = new DataView(
  //       positionAtMB.data.buffer,
  //       positionAtMB.data.byteOffset,
  //       positionAtMB.data.length
  //     );
  //     const positionAtMBClosedAt = positionAtMBData.getBigInt64(195, true);
  //     console.log(
  //       "Position Closed At (MB): ",
  //       new Date(Number(positionAtMBClosedAt) * 1000).toISOString()
  //     );

  //     const participantAccount = await program.account.participant.fetch(
  //       participantPda
  //     );
  //     const positionAccount = await program.account.position.fetch(positionPda);
  //     const leaderboardAccount = await program.account.leaderboard.fetch(
  //       leaderboardPda
  //     );

  //     logParticipant(participantAccount);
  //     logPosition(positionAccount);
  //     logLeaderboard(leaderboardAccount);
  //   });
  // });

  // it("Undelegate Participant", async () => {
  //   const delegated = await getDelegationStatus(
  //     routerConnection,
  //     participantPda
  //   );

  //   if (delegated.isDelegated) {
  //     console.log("Undelegating Participant...");
  //     const tx = await program.methods
  //       .undelegateParticipant(leaguePda)
  //       .accounts({
  //         user: anchor.Wallet.local().publicKey,
  //         // @ts-ignore
  //         participant: participantPda,
  //       })
  //       .transaction();

  //     const signature = await sendMagicTransaction(routerConnection, tx, [
  //       anchor.Wallet.local().payer,
  //     ]);

  //     console.log("✅ Undelegated Participant! Signature:", signature);
  //   } else {
  //     console.log(
  //       "ℹ️ Participant is not delegated, skipping undelegate participant"
  //     );
  //   }
  // });

  // it("Increment Counter!", async () => {
  //   const tx = await program.methods
  //     .increment()
  //     .accounts({
  //       counter: pda,
  //     })
  //     .transaction() as Transaction;

  //   const signature = await sendMagicTransaction(
  //     routerConnection,
  //     tx,
  //     [anchor.Wallet.local().payer]
  //   );
  //   console.log("✅ Incremented Counter PDA! Signature:", signature);
  // });

  // it("Update Leaderboard!", async () => {
  //   const tx = await program.methods
  //     .updateLeaderboard()
  //     .accounts({
  //       counter: pda,
  //       escrow: pda,
  //       escrowAuth: pda,
  //     })
  //     .transaction();

  //   const signature = await sendMagicTransaction(
  //     routerConnection,
  //     tx,
  //     [anchor.Wallet.local().payer]
  //   );

  //   await printCounter(program, pda, leaderboard_pda, routerConnection, signature, "✅ Updated Leaderboard!");
  // });

  // it("Delegate Counter to ER!", async () => {
  //   const validatorKey = await getClosestValidator(routerConnection);
  //   console.log("Delegating to closest validator: ", validatorKey.toString());

  //   const tx = await program.methods
  //     .delegate()
  //     .accounts({
  //       payer: anchor.Wallet.local().publicKey,
  //       pda: pda,
  //     })
  //     .transaction();

  //   const signature = await sendMagicTransaction(
  //     routerConnection,
  //     tx,
  //     [anchor.Wallet.local().payer]
  //   );

  //   await sleepWithAnimation(10); // ensure the delegation is processed
  //   console.log("✅ Delegated Counter PDA! Signature:", signature);
  // });

  // it("Increment Counter on ER!", async () => {
  //   const tx = await program.methods
  //     .increment()
  //     .accounts({
  //       counter: pda,
  //     })
  //     .transaction();

  //   const signature = await sendMagicTransaction(
  //     routerConnection,
  //     tx,
  //     [anchor.Wallet.local().payer]
  //   );

  //   await printCounter(program, pda, leaderboard_pda, routerConnection, signature, "✅ Incremented Counter PDA!");
  // });

  // it("Update Leaderboard While Delegated!", async () => {
  //   const tx = await program.methods
  //     .commitAndUpdateLeaderboard()
  //     .accounts({
  //       payer: anchor.Wallet.local().publicKey,
  //       programId: program.programId,
  //     })
  //     .transaction();

  //     const signature = await sendMagicTransaction(
  //       routerConnection,
  //       tx,
  //       [anchor.Wallet.local().payer]
  //     );

  //     await sleepWithAnimation(5);
  //     await printCounter(program, pda, leaderboard_pda, routerConnection, signature, "✅ Updated Leaderboard While Delegated!");
  // });

  // it("Undelegate Counter!", async () => {
  //   const tx = await program.methods
  //     .undelegate()
  //     .accounts({
  //       payer: anchor.Wallet.local().publicKey,
  //     })
  //     .transaction();

  //   const signature = await sendMagicTransaction(
  //     routerConnection,
  //     tx,
  //     [anchor.Wallet.local().payer]
  //   );
  //   await sleepWithAnimation(5);
  //   await printCounter(program, pda, leaderboard_pda, routerConnection, signature, "✅ Undelegated Counter PDA!");
  // });

  describe("Open Private Position Scenario", () => {
    before(async () => {
      console.log("Setting up Permission TEE AuthToken");
      authTokenAdmin = await getAuthToken(
        teeConnection.rpcEndpoint,
        anchor.Wallet.local().payer
      );
      console.log("Auth Token Admin: ", authTokenAdmin);
      providerTeeAdmin = new anchor.AnchorProvider(
        new anchor.web3.Connection(
          "https://tee.magicblock.app?token=" + authTokenAdmin,
          {
            wsEndpoint: "wss://tee.magicblock.app?token=" + authTokenAdmin,
          }
        ),
        anchor.Wallet.local()
      );
    });

    const permissionPda = permissionPdaFromAccount(privatePositionPda);
    const groupId = anchor.web3.Keypair.generate().publicKey;
    const groupPda = groupPdaFromId(groupId);

    console.log("================================================");
    console.log("Private Position PDA: ", privatePositionPda.toBase58());
    console.log("Permission PDA: ", permissionPda.toBase58());
    console.log("Group ID: ", groupId.toBase58());
    console.log("Group PDA: ", groupPda.toBase58());
    console.log("================================================");

    // it("Init, Create Permission, Delegate Position", async () => {
    //   const initIx = await program.methods
    //     .initUnopenedPosition(
    //       leaguePda,
    //       new anchor.BN(privatePositionSeq),
    //       marketPda,
    //       SOL_DECIMALS
    //     )
    //     .accounts({
    //       user: anchor.Wallet.local().publicKey,
    //       // @ts-ignore
    //       position: privatePositionPda,
    //       priceFeed: price_feed_pda,
    //       systemProgram: anchor.web3.SystemProgram.programId,
    //     })
    //     .instruction();

    //   const createPermissionIx = await program.methods
    //     .createPositionPermission(
    //       leaguePda,
    //       anchor.Wallet.local().publicKey,
    //       new anchor.BN(privatePositionSeq),
    //       groupId
    //     )
    //     .accountsPartial({
    //       payer: anchor.Wallet.local().publicKey,
    //       position: privatePositionPda,
    //       permission: permissionPda,
    //       group: groupPda,
    //       permissionProgram: PERMISSION_PROGRAM_ID,
    //       systemProgram: anchor.web3.SystemProgram.programId,
    //     })
    //     .remainingAccounts([
    //       {
    //         pubkey: anchor.Wallet.local().publicKey,
    //         isWritable: false,
    //         isSigner: true,
    //       },
    //     ])
    //     .instruction();

    //   const delegatePositionIx = await program.methods
    //     .delegateUnopenedPosition(participantPda, new anchor.BN(privatePositionSeq))
    //     .accounts({
    //       user: anchor.Wallet.local().publicKey,
    //       league: leaguePda,
    //       position: privatePositionPda,
    //     })
    //     .instruction();

    //   let tx = new anchor.web3.Transaction().add(
    //     initIx,
    //     createPermissionIx,
    //     delegatePositionIx
    //   );

    //   tx.feePayer = anchor.Wallet.local().publicKey;
    //   const txHash = await sendMagicTransaction(routerConnection, tx, [
    //     anchor.Wallet.local().payer,
    //   ]);
    //   console.log("✅ Opened Private Position! Signature:", txHash);

    //   await sleepWithAnimation(5);
    // });

    it("Open Private Position", async () => {
      const openIx = await program.methods
        .openPosition(
          new anchor.BN(privatePositionSeq),
          { long: {} },
          new anchor.BN(1000000),
          10
        )
        .accounts({
          user: anchor.Wallet.local().publicKey,
          // @ts-ignore
          position: privatePositionPda,
          priceFeed: price_feed_pda,
          league: leaguePda,
          market: marketPda,
          participant: participantPda,
        })
        .instruction();

      let tx = new anchor.web3.Transaction().add(openIx);
      tx.feePayer = anchor.Wallet.local().publicKey;
      tx.recentBlockhash = (
        await providerTeeAdmin.connection.getLatestBlockhash()
      ).blockhash;
      const txHash = await sendAndConfirmTransaction(
        providerTeeAdmin.connection,
        tx,
        [anchor.Wallet.local().payer]
      );

      console.log("Opened Private Position! Signature:", txHash);
    });

    it("Check Visibility of Position (Non-permissioned entity)", async () => {
      const accountInfo = await teeConnection.getAccountInfo(privatePositionPda);
      if (!accountInfo) {
        console.log("Passed! Position account not found");
        return;
      }
      const positionData = accountInfo.data;
      const positionAccount = program.account.position.coder.accounts.decode(
        "position",
        positionData
      );
      console.log("Position Account: ", positionAccount);
    });

    it("Check Visibility of Position (Permissioned entity)", async () => {
      const accountInfo = await providerTeeAdmin.connection.getAccountInfo(
        privatePositionPda
      );
      // Check providerTeeAdmin details
      console.log(
        "Provider Tee Admin: ",
        providerTeeAdmin.connection.rpcEndpoint
      );
      console.log(
        "Provider Tee Admin Public Key: ",
        providerTeeAdmin.publicKey.toBase58()
      );
      console.log("Private Position Account: ", accountInfo);
      const positionData = accountInfo.data;
      const positionAccount = program.account.position.coder.accounts.decode(
        "position",
        positionData
      );
      console.log("Position Account: ", positionAccount);
    });
  });

  describe("Private Resource Example", () => {
    

  });
});

// async function printCounter(program: Program<TdfProgram>, counter_pda: web3.PublicKey, leaderboard_pda: web3.PublicKey, routerConnection: web3.Connection, signature: string, message: string) {
//   console.log(message+" Signature: ", signature);
//   const delegationStatus = await getDelegationStatus(routerConnection, counter_pda);
//   const leaderboardAccount = await program.account.leaderboard.fetch(leaderboard_pda);

//   var counterER = "";
//   var counterBase = "";
//   var delegationStatusMsg = "";

//   if (delegationStatus.isDelegated) {
//     const counterAccountER = await routerConnection.getAccountInfo(counter_pda);
//     const countValue = counterAccountER?.data.readBigUInt64LE(8);
//     counterER = countValue?.toString() || "0";
//     counterBase = "<Delegated>";
//     delegationStatusMsg = "✅ Delegated";
//   } else {
//     counterER = "<Not Delegated>";
//     const counterAccount = await program.account.counter.fetch(counter_pda); // Fetchs on Devnet
//     counterBase = counterAccount.count.toNumber().toString();
//     delegationStatusMsg = "❌ Not Delegated";
//   }

//   console.log("--------------------------------");
//   console.log("| "+delegationStatusMsg);
//   console.log("--------------------------------");
//   console.log("| Counter (Base): ", counterBase);
//   console.log("| Counter (ER): ", counterER);
//   console.log("| High Score: ", leaderboardAccount.highScore.toNumber());
//   console.log("--------------------------------");

// }

async function sleepWithAnimation(seconds: number): Promise<void> {
  const totalMs = seconds * 1000;
  const interval = 500; // Update every 500ms
  const iterations = Math.floor(totalMs / interval);

  for (let i = 0; i < iterations; i++) {
    const dots = ".".repeat((i % 3) + 1);
    process.stdout.write(`\rWaiting${dots}   `);
    await new Promise((resolve) => setTimeout(resolve, interval));
  }

  process.stdout.write("\r\x1b[K");
  console.log();
}

function logParticipant(participant) {
  console.log(
    "Participant unrealized PnL: ",
    participant.unrealizedPnl.toString()
  );
  console.log(
    "Participant virtual balance: ",
    participant.virtualBalance.toString()
  );
  console.log("Participant used margin: ", participant.usedMargin.toString());
  console.log("Participant total volume: ", participant.totalVolume.toString());
}

function logPosition(position) {
  // log position details
  console.log(
    "Position opened at: ",
    new Date(position.openedAt.toNumber() * 1000).toISOString()
  );
  console.log(
    "Position closed at: ",
    new Date(position.closedAt.toNumber() * 1000).toISOString()
  );
  console.log("Position direction: ", position.direction.toString());
  console.log("Position entry price: ", position.entryPrice.toString());
  console.log("Position entry size: ", position.entrySize.toString());
  console.log("Position leverage: ", position.leverage.toString());
  console.log("Position size: ", position.size.toString());
  console.log("Position notional: ", position.notional.toString());
  console.log("Position closed size: ", position.closedSize.toString());
  console.log("Position closed price: ", position.closedPrice.toString());
  console.log("Position unrealized PnL: ", position.unrealizedPnl.toString());
  console.log("Position closed equity: ", position.closedEquity.toString());
  console.log("Position closed PnL: ", position.closedPnl.toString());
}

function logLeaderboard(leaderboard) {
  console.log(
    "Leaderboard topk equity: ",
    leaderboard.topkEquity.map((p) => p.toString())
  );
  console.log(
    "Leaderboard topk equity scores: ",
    leaderboard.topkEquityScores.map((p) => p.toString())
  );
}
