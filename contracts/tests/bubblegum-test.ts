import * as anchor from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BN, Provider, Program, AccountClient } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { Gummyroll } from "../target/types/gummyroll";
import { PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import { assert } from "chai";
import {
  createMintV1Instruction,
  createDecompressV1Instruction,
  createTransferInstruction,
  createDelegateInstruction,
  createRedeemInstruction,
  createCancelRedeemInstruction,
  createCreateTreeInstruction,
  MetadataArgs,
} from "../sdk/bubblegum/src/generated";

import { buildTree, Tree } from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  getRootOfOnChainMerkleRoot,
  assertOnChainMerkleRollProperties,
  createTransferAuthorityIx,
  createAllocTreeIx,
} from "../sdk/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} from "@solana/spl-token";

import { bufferToArray, execute } from "./utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "../sdk/utils";
import { getBubblegumAuthorityPDA, getCreateTreeIxs, getNonceCount, getVoucherPDA } from "../sdk/bubblegum/src/convenience";
import { TokenProgramVersion } from "../sdk/bubblegum/src/generated";
import { createRemoveAppendAuthorityInstruction } from "../sdk/bubblegum/src/generated/instructions/removeAppendAuthority";
import { sleep } from "@metaplex-foundation/amman/dist/utils";
import { createSetAppendAuthorityInstruction } from "../sdk/bubblegum/src/generated/instructions/setAppendAuthority";

// @ts-ignore
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;

function createExampleMetadata(name: string, symbol: string, uri: string): MetadataArgs {
  return {
    name,
    symbol,
    uri,
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: TokenProgramVersion.Original,
    collection: null,
    uses: null,
    creators: [],
  };
}

async function getNonceAccount(bubblegum: Program<Bubblegum>): Promise<PublicKey> {
  let [nonce] = await PublicKey.findProgramAddress(
    [Buffer.from("bubblegum")],
    bubblegum.programId
  );
  return nonce;
}

async function getTreeAuthority(bubblegum: Program<Bubblegum>, merkleSlab: PublicKey): Promise<PublicKey> {
  let [authority] = await PublicKey.findProgramAddress(
    [merkleSlab.toBuffer()],
    bubblegum.programId
  );
  return authority;
}

type Version = {
  v0: Object,
}

// async function createMintIx(bubblegum: Program<Bubblegum>,
//   metadata: any,
//   version: Version,
//   payer: Keypair,
//   owner: PublicKey,
//   delegate: PublicKey,
//   merkleSlab: PublicKey,
//   mintAuthority: PublicKey
// ): Promise<TransactionInstruction> {
//   return bubblegum.instruction.mintV1(version, metadata, {
//     accounts: {
//       mintAuthority,
//       authority: await getTreeAuthority(bubblegum, merkleSlab),
//       gummyrollProgram: GummyrollProgramId,
//       candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
//       owner,
//       delegate,
//       merkleSlab,
//     },
//     signers: [payer],
//   });
// }

// function createRemoveAppendAuthorityIx(
//   bubblegum: Program<Bubblegum>,
//   appendAuthority: Keypair,
//   authorityToRemove: PublicKey,
//   authority: PublicKey,
// ): TransactionInstruction {
//   return bubblegum.instruction.removeAppendAuthority({
//     accounts: {
//       appendAuthority: appendAuthority.publicKey,
//       authorityToRemove,
//       authority,
//     },
//     signers: [appendAuthority]
//   })
// }

// function createSetAppendAuthorityIx(
//   bubblegum: Program<Bubblegum>,
//   index: number,
//   treeDelegate: Keypair,
//   newAppendAuthority: PublicKey,
//   authority: PublicKey,
// ): TransactionInstruction {
//   return bubblegum.instruction.setAppendAuthority(
//     index,
//     {
//       accounts: {
//         treeDelegate: treeDelegate.publicKey,
//         newAppendAuthority,
//         authority,
//       },
//       signers: [treeDelegate]
//     }
//   );
// }

describe("bubblegum", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let treeAuthority: PublicKey;
  let merkleRollKeypair: Keypair;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;

  let payer = Keypair.generate();
  let destination = Keypair.generate();
  let delegateKey = Keypair.generate();
  let connection = new web3Connection("http://localhost:8899", {
    commitment: "confirmed",
  });
  let wallet = new NodeWallet(payer);
  anchor.setProvider(
    new Provider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    })
  );
  Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;

  async function createTreeOnChain(
    payer: Keypair,
    destination: Keypair,
    delegate: Keypair
  ): Promise<[Keypair, Tree, PublicKey]> {
    const merkleRollKeypair = Keypair.generate();

    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(payer.publicKey, 2e9),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        destination.publicKey,
        2e9
      ),
      "confirmed"
    );
    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(
        delegate.publicKey,
        2e9
      ),
      "confirmed"
    );
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    const tree = buildTree(leaves);

    let tx = new Transaction();
    const ixs = await getCreateTreeIxs(connection, MAX_DEPTH, MAX_SIZE, 0, payer.publicKey, merkleRollKeypair.publicKey, payer.publicKey);
    await execute(Bubblegum.provider, ixs, [payer, merkleRollKeypair], true);

    const authority = await getBubblegumAuthorityPDA(merkleRollKeypair.publicKey);
    await assertOnChainMerkleRollProperties(
      Bubblegum.provider.connection,
      MAX_DEPTH,
      MAX_SIZE,
      authority,
      new PublicKey(tree.root),
      merkleRollKeypair.publicKey
    );

    return [merkleRollKeypair, tree, authority];
  }

  describe("Testing bubblegum", () => {
    beforeEach(async () => {
      let [computedMerkleRoll, computedOffChainTree, computedTreeAuthority] =
        await createTreeOnChain(payer, destination, delegateKey);
      merkleRollKeypair = computedMerkleRoll;
      offChainTree = computedOffChainTree;
      treeAuthority = computedTreeAuthority;
    });
    it("Mint to tree", async () => {
      const metadata: MetadataArgs = createExampleMetadata("test", "test", "www.solana.com");
      const mintIx = createMintV1Instruction(
        {
          mintAuthority: payer.publicKey,
          authority: treeAuthority,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        { message: metadata }
      );
      console.log(" - Minting to tree");
      await execute(Bubblegum.provider, [mintIx], [payer], true);

      console.log(" - Transferring Ownership");
      const dataHash = bufferToArray(
        Buffer.from(keccak_256.digest(mintIx.data.slice(8)))
      );
      const creatorHash = bufferToArray(Buffer.from(keccak_256.digest([])));
      let onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);
      const nonceCount = await getNonceCount(Bubblegum.provider.connection, merkleRollKeypair.publicKey);
      const leafNonce = nonceCount.sub(new BN(1));
      let transferIx = createTransferInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          newOwner: destination.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      await execute(Bubblegum.provider, [transferIx], [payer], true);

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      console.log(" - Delegating Ownership");
      let delegateIx = await createDelegateInstruction(
        {
          authority: treeAuthority,
          owner: destination.publicKey,
          previousDelegate: destination.publicKey,
          newDelegate: delegateKey.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      await execute(Bubblegum.provider, [delegateIx], [destination]);

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      console.log(" - Transferring Ownership (through delegate)");
      let delTransferIx = createTransferInstruction(
        {
          authority: treeAuthority,
          owner: destination.publicKey,
          delegate: delegateKey.publicKey,
          newOwner: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      delTransferIx.keys[2].isSigner = true;
      let delTransferTx = await Bubblegum.provider.send(
        new Transaction().add(delTransferIx),
        [delegateKey],
        {
          skipPreflight: true,
          commitment: "confirmed",
        }
      );

      onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);

      let voucher = await getVoucherPDA(
        Bubblegum.provider.connection,
        merkleRollKeypair.publicKey,
        0,
      );

      console.log(" - Redeeming Leaf", voucher.toBase58());
      let redeemIx = createRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: new BN(0),
          index: 0,
        }
      );
      let redeemTx = await Bubblegum.provider.send(
        new Transaction().add(redeemIx),
        [payer],
        {
          skipPreflight: true,
          commitment: "confirmed",
        }
      );
      console.log(" - Cancelling redeem (reinserting to tree)");

      const cancelRedeemIx = createCancelRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
        }
      );
      let cancelRedeemTx = await Bubblegum.provider.send(
        new Transaction().add(cancelRedeemIx),
        [payer],
        {
          commitment: "confirmed",
        }
      );

      console.log(" - Decompressing leaf");

      redeemIx = createRedeemInstruction(
        {
          authority: treeAuthority,
          owner: payer.publicKey,
          delegate: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          gummyrollProgram: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          voucher: voucher,
        },
        {
          root: bufferToArray(onChainRoot),
          dataHash,
          creatorHash,
          nonce: leafNonce,
          index: 0,
        }
      );
      let redeemTx2 = await Bubblegum.provider.send(
        new Transaction().add(redeemIx),
        [payer],
        {
          commitment: "confirmed",
        }
      );

      let voucherData = await Bubblegum.account.voucher.fetch(voucher);

      let [asset] = await PublicKey.findProgramAddress(
        [
          Buffer.from("asset"),
          merkleRollKeypair.publicKey.toBuffer(),
          leafNonce.toBuffer("le", 8),
        ],
        Bubblegum.programId
      );

      let [mintAuthority] = await PublicKey.findProgramAddress(
        [asset.toBuffer()],
        Bubblegum.programId
      );

      const getMetadata = async (
        mint: anchor.web3.PublicKey
      ): Promise<anchor.web3.PublicKey> => {
        return (
          await anchor.web3.PublicKey.findProgramAddress(
            [Buffer.from("metadata"), PROGRAM_ID.toBuffer(), mint.toBuffer()],
            PROGRAM_ID
          )
        )[0];
      };

      const getMasterEdition = async (
        mint: anchor.web3.PublicKey
      ): Promise<anchor.web3.PublicKey> => {
        return (
          await anchor.web3.PublicKey.findProgramAddress(
            [
              Buffer.from("metadata"),
              PROGRAM_ID.toBuffer(),
              mint.toBuffer(),
              Buffer.from("edition"),
            ],
            PROGRAM_ID
          )
        )[0];
      };

      let decompressIx = createDecompressV1Instruction(
        {
          voucher: voucher,
          owner: payer.publicKey,
          tokenAccount: await Token.getAssociatedTokenAddress(
            ASSOCIATED_TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
            asset,
            payer.publicKey
          ),
          mint: asset,
          mintAuthority: mintAuthority,
          metadata: await getMetadata(asset),
          masterEdition: await getMasterEdition(asset),
          sysvarRent: SYSVAR_RENT_PUBKEY,
          tokenMetadataProgram: PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        },
        {
          metadata,
        }
      );

      let decompressTx = await Bubblegum.provider.send(
        new Transaction().add(decompressIx),
        [payer],
        {
          commitment: "confirmed",
        }
      );
    });
    it("Test that allowlist prevents mints", async () => {
      const removeAppendAuthorityIx = createRemoveAppendAuthorityInstruction({
        authorityToRemove: payer.publicKey,
        appendAuthority: payer.publicKey,
        authority: treeAuthority,
      });
      await execute(Bubblegum.provider, [removeAppendAuthorityIx], [payer], true);

      const metadata = createExampleMetadata("racecar", "test", "www.solana.com")
      const mintIx = await createMintV1Instruction({
        mintAuthority: payer.publicKey,
        authority: treeAuthority,
        candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        gummyrollProgram: GummyrollProgramId,
        owner: payer.publicKey,
        delegate: payer.publicKey,
        merkleSlab: merkleRollKeypair.publicKey
      },
        {
          message: metadata,
        }
      );
      try {
        await execute(Bubblegum.provider, [, mintIx], [payer], true, true);
        assert(false, "This transaction should have failed since the payer is no longer append authority");
      } catch (e) {
        assert(true, "Successfully prevented payer from minting");
      }
    });
    it("Concurrently append into tree", async () => {

      // Create 5 random keys, set them as append publickeys
      const ALLOWLIST_SIZE = 5;
      const authIxs = [];
      const appendIxs = [];
      const keypairs = [];
      for (let i = 0; i < ALLOWLIST_SIZE; i++) {
        const keypair = Keypair.generate();
        const setAppendAuthorityIx = createSetAppendAuthorityInstruction(
          {
            treeDelegate: payer.publicKey,
            newAppendAuthority: keypair.publicKey,
            authority: treeAuthority,
          },
          {
            index: i,
          }
        );
        const metadata = createExampleMetadata(`${i}`, `${i}`, "www.solana.com");
        const mintIx = await createMintV1Instruction(
          {
            mintAuthority: keypair.publicKey,
            authority: treeAuthority,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            gummyrollProgram: GummyrollProgramId,
            owner: payer.publicKey,
            delegate: payer.publicKey,
            merkleSlab: merkleRollKeypair.publicKey,
          },
          {
            message: metadata,
          }
        );
        keypairs.push(keypair);
        authIxs.push(setAppendAuthorityIx);
        appendIxs.push(mintIx);
      }

      // All appends should succeed
      const allIxs = [].concat(authIxs, appendIxs);
      const allKeypairs = [payer].concat(keypairs);
      await execute(Bubblegum.provider, allIxs, allKeypairs, true);
    });
  });
});
