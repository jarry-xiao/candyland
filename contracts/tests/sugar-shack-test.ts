import * as anchor from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BN, Provider, Program } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
  AccountMeta,
} from "@solana/web3.js";
import { assert } from "chai";
import {
  createCreateTreeInstruction,
  createMintV1Instruction,
} from '../sdk/bubblegum/src/generated/instructions';
import {
  MarketplaceProperties
} from "../sdk/sugar-shack/src/generated/accounts/index";
import {
  createInitializeMarketplaceInstruction,
  createCreateOrModifyListingInstruction,
  createRemoveListingInstruction,
  createPurchaseInstruction,
  createWithdrawFeesInstruction
} from "../sdk/sugar-shack/src/generated/instructions";
import {
  getListingPDAKeyForPrice
} from "../sdk/sugar-shack";
import {
  CANDY_WRAPPER_PROGRAM_ID,
  getBubblegumAuthorityPDAKey
} from "../sdk/utils/index";
import {
  MetadataArgs,
  LeafSchema,
  leafSchemaBeet
} from "../sdk/bubblegum/src/generated/types";
import {
  createAllocTreeIx,
  getMerkleRollAccountSize,
  getRootOfOnChainMerkleRoot
} from "../sdk/gummyroll";
import {
  buildTree,
  hash,
  getProofOfLeaf,
  updateTree,
  Tree,
  TreeNode,
} from "./merkle-tree";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { execute, logTx, bufferToArray } from "./utils";
import { TokenProgramVersion, Version } from "../sdk/bubblegum/src/generated";
import { SugarShack } from "../target/types/sugar_shack";

// @ts-ignore
let BubblegumProgramId;
let GummyrollProgramId;

describe("sugar-shack", () => {
  // In this test, payer is the merkle tree admin and payer for all txs.
  const payer = Keypair.generate();

  // Establish connection to localcluster
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

  const SugarShack = anchor.workspace.SugarShack as Program<SugarShack>;
  BubblegumProgramId = anchor.workspace.Bubblegum.programId;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;

  describe("test sugarshack", async () => {
    let marketplaceAccountKey: PublicKey;
    let marketplaceShareRecipient: Keypair;
    let marketplaceAuthority: Keypair;
    let merkleRollKeypair: Keypair;
    let lister: Keypair;
    let bubblegumAuthority: PublicKey;
    let dataHashOfCompressedNFT: number[];
    let creatorHashOfCompressedNFT: number[];
    let leafNonce: BN;
    let listingPDAKey: PublicKey;
    let bufferOfProjectDropCreatorShare: Buffer;
    let projectDropCreator: Keypair;
    let listingPrice: BN;
    let compressedNFTMetadata: MetadataArgs;
    let originalProofToNFTLeaf: AccountMeta[];
    const marketplaceRoyaltyShare = 100;

    async function createOrModifyListing(
      priceForListing: BN,
      currentNFTOwner: Keypair,
      previousNFTDelegate: PublicKey,
      proofToLeaf: AccountMeta[] = null,
    ) {
      const onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);
      const newListingPDAKey = await getListingPDAKeyForPrice(priceForListing, SugarShack.programId);
      const createOrModifyListingIx = createCreateOrModifyListingInstruction(
        {
          owner: currentNFTOwner.publicKey,
          formerDelegate: previousNFTDelegate,
          newDelegate: newListingPDAKey,
          bubblegumAuthority,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          price: priceForListing,
          dataHash: dataHashOfCompressedNFT,
          creatorHash: creatorHashOfCompressedNFT,
          nonce: leafNonce,
          index: 0,
          root: bufferToArray(onChainRoot),
        }
      );
      if (proofToLeaf) {
        proofToLeaf.forEach(acctMeta => createOrModifyListingIx.keys.push(acctMeta));
      }
      await SugarShack.provider.send(new Transaction().add(createOrModifyListingIx), [currentNFTOwner], {
        commitment: "confirmed",
      });
    }

    async function removeListing(
      currentNFTOwner: Keypair,
      previousNFTDelegate: PublicKey,
      desiredDelegate: PublicKey,
      proofToLeaf: AccountMeta[] = null
    ) {
      const onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);
      const removeListIx = createRemoveListingInstruction(
        {
          owner: currentNFTOwner.publicKey,
          formerDelegate: previousNFTDelegate,
          newDelegate: desiredDelegate,
          bubblegumAuthority,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          dataHash: dataHashOfCompressedNFT,
          creatorHash: creatorHashOfCompressedNFT,
          nonce: leafNonce,
          index: 0,
          root: bufferToArray(onChainRoot),
        }
      );
      if (proofToLeaf) {
        proofToLeaf.forEach(acctMeta => removeListIx.keys.push(acctMeta));
      }
      await SugarShack.provider.send(new Transaction().add(removeListIx), [currentNFTOwner], {
        commitment: "confirmed",
      });
    }

    async function withdrawFees(feePayoutRecipient: PublicKey, authority: Keypair, lamportsToWithdraw: BN) {
      const withdrawFeesIx = createWithdrawFeesInstruction(
        {
          feePayoutRecipient,
          authority: authority.publicKey,
          marketplaceProps: marketplaceAccountKey,
          sysvarRent: SYSVAR_RENT_PUBKEY
        },
        {
          lamportsToWithdraw,
        }
      );
      await SugarShack.provider.send(new Transaction().add(withdrawFeesIx), [authority], {
        commitment: "confirmed",
      });
    }

    async function purchaseNFTFromListing(
      purchasePrice: BN,
      nftPurchaser: Keypair,
      proofToLeaf: AccountMeta[] = null,
    ) {
      let onChainRoot = await getRootOfOnChainMerkleRoot(connection, merkleRollKeypair.publicKey);
      let listedNFTDelegateKey: PublicKey = await getListingPDAKeyForPrice(purchasePrice, SugarShack.programId);
      const purchaseIx = createPurchaseInstruction(
        {
          formerOwner: lister.publicKey,
          purchaser: nftPurchaser.publicKey,
          listingDelegate: listedNFTDelegateKey,
          bubblegumAuthority,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId,
          marketplaceProps: marketplaceAccountKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        },
        {
          price: purchasePrice,
          dataHash: dataHashOfCompressedNFT,
          nonce: leafNonce,
          index: 0,
          root: bufferToArray(onChainRoot),
          creatorShares: bufferOfProjectDropCreatorShare,
        }
      );
      let remainingAccount = {
        pubkey: projectDropCreator.publicKey,
        isSigner: false,
        isWritable: true,
      };
      purchaseIx.keys.push(remainingAccount);
      if (proofToLeaf) {
        proofToLeaf.forEach(acctMeta => purchaseIx.keys.push(acctMeta));
      }
      let tx = new Transaction().add(purchaseIx);
      await SugarShack.provider.send(tx, [nftPurchaser], {
        commitment: "confirmed",
      });
      // For reference, a tree with depth 20 has a transaction size of 1123 with one creator and a canopy of depth 5
      //                a tree with depth 24 has a transaction size of 1255 with one creator and a canopy of depth 5
      let txSize = tx.serialize().length;
      console.log("Transaction Size", txSize);
    }

    before(async () => {
      // Fund the payer for the entire suite
      await SugarShack.provider.connection.confirmTransaction(
        await SugarShack.provider.connection.requestAirdrop(payer.publicKey, 75e9),
        "confirmed"
      );

      // Setup one-time state that will be shared among tests: Marketplace Properties account, Nonce if not already init by another test

      // Initialize marketplace properties account
      marketplaceAuthority = Keypair.generate();
      [marketplaceAccountKey] = await PublicKey.findProgramAddress(
        [Buffer.from("mymarketplace")],
        SugarShack.programId
      );
      let initMarketplacePropsIx = createInitializeMarketplaceInstruction(
        {
          marketplaceProps: marketplaceAccountKey,
          payer: payer.publicKey,
        },
        {
          royaltyShare: marketplaceRoyaltyShare,
          authority: marketplaceAuthority.publicKey,
        }
      );

      const initMarketplacePropsTx = new Transaction().add(initMarketplacePropsIx);
      await SugarShack.provider.send(initMarketplacePropsTx, [payer], {
        commitment: "confirmed",
      });

      // Confirm that properties of the onchain marketplace PDA match expectation
      const onChainMarketplaceAccount: MarketplaceProperties = await MarketplaceProperties.fromAccountAddress(SugarShack.provider.connection, marketplaceAccountKey);
      assert(
        onChainMarketplaceAccount.authority.equals(marketplaceAuthority.publicKey),
        "onchain marketplace account receiver does not match expectation"
      );
      assert(
        onChainMarketplaceAccount.share === marketplaceRoyaltyShare,
        "onchain marketplace account share does not match expectation"
      );
    });

    describe("core instructions", async () => {
      beforeEach(async () => {
        // Setup unique state for each test: a new merkle roll tree with a new NFT in it
        lister = Keypair.generate();
        merkleRollKeypair = Keypair.generate();
        const MERKLE_ROLL_MAX_DEPTH = 20;
        const MERKLE_ROLL_MAX_BUFFER_SIZE = 2048;

        // Make use of CANOPY to enable larger project sizes and give more breathing room for additional accounts in marketplace instructions
        const MERKLE_ROLL_CANOPY_DEPTH = 5;

        // Create the compressed NFT tree
        // Instruction to alloc new merkle roll account
        const allocMerkleRollAcctInstr = await createAllocTreeIx(
          SugarShack.provider.connection,
          MERKLE_ROLL_MAX_BUFFER_SIZE,
          MERKLE_ROLL_MAX_DEPTH,
          MERKLE_ROLL_CANOPY_DEPTH,
          payer.publicKey,
          merkleRollKeypair.publicKey,
        )
        bubblegumAuthority = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey, BubblegumProgramId);

        // Instruction to create merkle tree for compressed NFTs through Bubblegum
        const createCompressedNFTTreeIx = createCreateTreeInstruction(
          {
            treeCreator: payer.publicKey,
            payer: payer.publicKey,
            authority: bubblegumAuthority,
            gummyrollProgram: GummyrollProgramId,
            merkleSlab: merkleRollKeypair.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          },
          {
            maxDepth: MERKLE_ROLL_MAX_DEPTH,
            maxBufferSize: MERKLE_ROLL_MAX_BUFFER_SIZE
          }
        );
        let createCompressedNFTTreeTx = new Transaction().add(allocMerkleRollAcctInstr).add(createCompressedNFTTreeIx);
        await SugarShack.provider.send(createCompressedNFTTreeTx, [payer, merkleRollKeypair], {
          commitment: "confirmed",
        });

        // build a corresponding off-chain tree...this allows us to fetch a proof
        const leaves = Array(2 ** MERKLE_ROLL_MAX_DEPTH).fill(Buffer.alloc(32));
        const tree = buildTree(leaves);

        // @dev: notice that even as the hash of the leaf changes, we continue passing in the same stale proof to the original leaf
        //       before it was even listed. This works because Gummyroll has a fallback mechanism to reproduce a valid proof from the 
        //       beginning of its buffer if the supplied proof is invalid. Thus, this allows us to present a proof of accurate *size*
        //       without needing to locally track how the proof actually changes which is harder with local tests. Note though, that after
        //       more than MAX_BUFFER_SIZE operations this is no longer valid, and in general is not good practice for a marketplace with
        //       access to indexing infra.
        originalProofToNFTLeaf = getProofOfLeaf(tree, (2 ** MERKLE_ROLL_MAX_DEPTH) - 1).slice(0, -1 * MERKLE_ROLL_CANOPY_DEPTH).map((node) => {
          return {
            pubkey: new PublicKey(node.node),
            isSigner: false,
            isWritable: false,
          };
        });

        projectDropCreator = Keypair.generate();

        // Mint an NFT to the tree, NFT to be owned by "lister"
        compressedNFTMetadata = {
          name: "test",
          symbol: "test",
          uri: "www.solana.com",
          sellerFeeBasisPoints: 0,
          primarySaleHappened: false,
          isMutable: false,
          editionNonce: null,
          tokenStandard: null,
          tokenProgramVersion: TokenProgramVersion.Original,
          collection: null,
          uses: null,
          creators: [{ address: projectDropCreator.publicKey, verified: true, share: 5 }],
        };
        const mintIx = createMintV1Instruction({
          mintAuthority: payer.publicKey,
          authority: bubblegumAuthority,
          gummyrollProgram: GummyrollProgramId,
          owner: lister.publicKey,
          delegate: lister.publicKey,
          merkleSlab: merkleRollKeypair.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        }, { message: compressedNFTMetadata });

        await SugarShack.provider.send(new Transaction().add(mintIx), [payer],
          {
            skipPreflight: true,
            commitment: "confirmed",
          }
        );

        // Get data_hash and creator_hash information for future transactions
        let bufferOfProjectDropCreatorPubkey = projectDropCreator.publicKey.toBuffer();
        bufferOfProjectDropCreatorShare = Buffer.from([compressedNFTMetadata.creators[0].share]);
        let bufferOfCreatorData = Buffer.concat([bufferOfProjectDropCreatorPubkey, bufferOfProjectDropCreatorShare]);
        dataHashOfCompressedNFT = bufferToArray(Buffer.from(keccak_256.digest(mintIx.data.slice(8))));
        creatorHashOfCompressedNFT = bufferToArray(Buffer.from(keccak_256.digest(bufferOfCreatorData)));

        // Get the nonce for the minted leaf
        const nonceInfo = await SugarShack.provider.connection.getAccountInfo(bubblegumAuthority);
        leafNonce = (new BN(nonceInfo.data.slice(8, 16), "le")).sub(new BN(1));

        // Record the PDA key that will be used as the "default" listing for each test
        listingPrice = new BN(1 * LAMPORTS_PER_SOL);
        listingPDAKey = await getListingPDAKeyForPrice(listingPrice, SugarShack.programId);

        await createOrModifyListing(new BN(1 * LAMPORTS_PER_SOL), lister, lister.publicKey, originalProofToNFTLeaf);
      });
      it("can modify listing", async () => {
        // Modify listing to have price 654321
        await createOrModifyListing(new BN(654321), lister, listingPDAKey, originalProofToNFTLeaf);

        // We can demonstrate that the modification worked by demonstrating that modifying using the old listingPDAKey will now fail
        try {
          await createOrModifyListing(new BN(555333), lister, listingPDAKey, originalProofToNFTLeaf);
          assert(false, "Was able to update listing despite earlier modification of delegate key!")
        } catch (e) { }
      });
      it("can remove listing", async () => {
        await removeListing(lister, listingPDAKey, lister.publicKey, originalProofToNFTLeaf);

        // Purchase after listing removal fails
        let nftPurchaser = Keypair.generate();
        await SugarShack.provider.connection.confirmTransaction(
          await SugarShack.provider.connection.requestAirdrop(nftPurchaser.publicKey, 2 * LAMPORTS_PER_SOL),
          "confirmed"
        );

        try {
          await purchaseNFTFromListing(listingPrice, nftPurchaser, originalProofToNFTLeaf);
          assert(false, "Unexpectedly, purchasing NFT after listing removal succeeded");
        } catch (e) { }
      });
      it("can purchase listed NFT", async () => {

        // Create and fund the purchaser account
        let nftPurchaser = Keypair.generate();
        await SugarShack.provider.connection.confirmTransaction(
          await SugarShack.provider.connection.requestAirdrop(nftPurchaser.publicKey, 2 * LAMPORTS_PER_SOL),
          "confirmed"
        );
        const originalMarketplacePDABalance = await SugarShack.provider.connection.getBalance(marketplaceAccountKey);
        await purchaseNFTFromListing(listingPrice, nftPurchaser, originalProofToNFTLeaf);

        // Assert on expected balance changes after NFT purchase
        const expectedMarketplaceFeePayout = listingPrice.toNumber() * marketplaceRoyaltyShare / 10000;
        assert(
          originalMarketplacePDABalance + expectedMarketplaceFeePayout === await SugarShack.provider.connection.getBalance(marketplaceAccountKey),
          "Marketplace did not recieve expected royalty"
        );

        const remainingLamportsToPayout = listingPrice.toNumber() - expectedMarketplaceFeePayout;
        const expectedCreatorPayout = remainingLamportsToPayout * compressedNFTMetadata.creators[0].share / 100;
        assert(
          expectedCreatorPayout === await SugarShack.provider.connection.getBalance(projectDropCreator.publicKey),
          "Creator did not recieve expected royalty"
        );
        const expectedListerPayout = remainingLamportsToPayout - expectedCreatorPayout;
        assert(
          expectedListerPayout === await SugarShack.provider.connection.getBalance(lister.publicKey),
          "Lister did not recieve expected royalty"
        );
        assert(
          (2 * LAMPORTS_PER_SOL) - listingPrice.toNumber() === await SugarShack.provider.connection.getBalance(nftPurchaser.publicKey),
          "NFT purchaser balance did not change as expected"
        );

        // Create marketplace share recipient account
        marketplaceShareRecipient = Keypair.generate();
        // Marketplace can now withdraw fee payout to external wallet
        await withdrawFees(marketplaceShareRecipient.publicKey, marketplaceAuthority, new BN(expectedMarketplaceFeePayout));

        // Assert that fee withdrawal occurred as expected
        assert(
          expectedMarketplaceFeePayout === await SugarShack.provider.connection.getBalance(marketplaceShareRecipient.publicKey),
          "Marketplace share RECIPIENT balance did not increment as expected after fee withdrawal"
        );
        assert(
          originalMarketplacePDABalance === await SugarShack.provider.connection.getBalance(marketplaceAccountKey),
          "Marketplace PDA balance did not decrease as expected after fee withdrawal"
        );

        // Purchaser is now able to list NFT
        await createOrModifyListing(new BN(654321), nftPurchaser, nftPurchaser.publicKey, originalProofToNFTLeaf);
      });
    });
  });
});
