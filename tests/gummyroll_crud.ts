import * as anchor from "@project-serum/anchor";
import {
    Keypair,
    Transaction,
    SystemProgram,
    PublicKey,
} from "@solana/web3.js";
import { assert, expect } from "chai";
import { Gummyroll } from "../target/types/gummyroll";
import { GummyrollCrud } from "../target/types/gummyroll_crud";
import { Program } from "@project-serum/anchor";
import {
    decodeMerkleRoll,
    getMerkleRollAccountSize,
} from "./merkle-roll-serde";
import { buildTree, getProofOfLeaf, hash, updateTree } from "./merkle-tree";

// @ts-ignore
const Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;
// @ts-ignore
const GummyrollCrud = anchor.workspace.GummyrollCrud as Program<GummyrollCrud>;

const connection = GummyrollCrud.provider.connection;
describe("Gummyroll CRUD program", () => {
    const MAX_DEPTH = 14;
    const MAX_SIZE = 64;
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);

    let tree: ReturnType<typeof buildTree>;

    async function getActualRoot() {
        const merkleRollAccount =
            await Gummyroll.provider.connection.getAccountInfo(
                merkleRollKeypair.publicKey
            );
        const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
        return merkleRoll.roll.changeLogs[
            merkleRoll.roll.activeIndex
            ].root.toBuffer();
    }
    async function appendMessage(message: string) {
        const addIx = GummyrollCrud.instruction.add(Buffer.from(message), {
            accounts: {
                gummyrollProgram: Gummyroll.programId,
                merkleRoll: merkleRollKeypair.publicKey,
                owner: feePayerKeypair.publicKey,
            },
            signers: [feePayerKeypair],
        });
        await GummyrollCrud.provider.send(
            new Transaction().add(addIx),
            [feePayerKeypair],
            {
                commitment: "confirmed",
            }
        );
    }
    function recomputeRootByAddingLeafToTreeWithMessageAtIndex(
        owner: PublicKey,
        message: string,
        index: number
    ) {
        const newLeaf = hash(owner.toBuffer(), Buffer.from(message));
        updateTree(tree, newLeaf, index);
        return tree.root;
    }
    function recomputeRootByRemovingLeafFromTreeAtIndex(index: number) {
        const newLeaf = Buffer.alloc(32, 0);
        updateTree(tree, newLeaf, index);
        return tree.root;
    }
    let feePayerKeypair: Keypair;
    let merkleRollKeypair: Keypair;
    beforeEach(async () => {
        const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
        tree = buildTree(leaves);

        feePayerKeypair = Keypair.generate();
        merkleRollKeypair = Keypair.generate();
        await Gummyroll.provider.connection.confirmTransaction(
            await Gummyroll.provider.connection.requestAirdrop(
                feePayerKeypair.publicKey,
                2e9
            ),
            "confirmed"
        );
        const allocGummyrollAccountIx = SystemProgram.createAccount({
            fromPubkey: feePayerKeypair.publicKey,
            newAccountPubkey: merkleRollKeypair.publicKey,
            lamports:
                await Gummyroll.provider.connection.getMinimumBalanceForRentExemption(
                    requiredSpace
                ),
            space: requiredSpace,
            programId: Gummyroll.programId,
        });
        const initGummyrollTx = Gummyroll.instruction.initEmptyGummyroll(
            MAX_DEPTH,
            MAX_SIZE,
            {
                accounts: {
                    authority: feePayerKeypair.publicKey,
                    merkleRoll: merkleRollKeypair.publicKey,
                },
                signers: [feePayerKeypair],
            }
        );
        const tx = new Transaction()
            .add(allocGummyrollAccountIx)
            .add(initGummyrollTx);
        const initGummyRollTxId = await Gummyroll.provider.send(
            tx,
            [feePayerKeypair, merkleRollKeypair],
            {
                commitment: "confirmed",
            }
        );
        assert(initGummyRollTxId, "Failed to initialize an empty Gummyroll");
    });
    describe("`Add` instruction", () => {
        describe("having appended the first item", () => {
            const firstTestMessage = "First test message";
            beforeEach(async () => {
                await appendMessage(firstTestMessage);
            });
            it("updates the root hash correctly", async () => {
                const actualRoot = await getActualRoot();
                const expectedRoot = recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                    feePayerKeypair.publicKey,
                    firstTestMessage,
                    0
                );
                expect(expectedRoot.compare(actualRoot)).to.equal(
                    0,
                    "On-chain root hash does not equal expected hash"
                );
            });
            describe("having appended the second item", () => {
                const secondTestMessage = "Second test message";
                beforeEach(async () => {
                    await appendMessage(secondTestMessage);
                });
                it("updates the root hash correctly", async () => {
                    const actualRoot = await getActualRoot();
                    recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                        feePayerKeypair.publicKey,
                        firstTestMessage,
                        0
                    );
                    const expectedRoot =
                        recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                            feePayerKeypair.publicKey,
                            secondTestMessage,
                            1
                        );
                    expect(expectedRoot.compare(actualRoot)).to.equal(
                        0,
                        "On-chain root hash does not equal expected hash"
                    );
                });
            });
        });
    });
    describe("`Transfer` instruction", () => {
        const message = "Message";
        async function transferMessage(
            newOwnerPubkey: PublicKey,
            index: number,
            config: { overrides?: { message?: string; signer?: Keypair } } = {}
        ) {
            const proofPubkeys = getProofOfLeaf(tree, index).map(({ node }) => ({
                pubkey: new PublicKey(node),
                isSigner: false,
                isWritable: false,
            }));
            const signer = config.overrides?.signer;
            const transferIx = GummyrollCrud.instruction.transfer(
                Buffer.from(tree.root, 0, 32),
                Buffer.from(config.overrides?.message ?? message),
                0,
                {
                    accounts: {
                        gummyrollProgram: Gummyroll.programId,
                        merkleRoll: merkleRollKeypair.publicKey,
                        newOwner: newOwnerPubkey,
                        owner: feePayerKeypair.publicKey,
                    },
                    signers: [signer ?? feePayerKeypair],
                    remainingAccounts: proofPubkeys,
                }
            );
            const tx = new Transaction().add(transferIx);
            await GummyrollCrud.provider.send(tx, [signer ?? feePayerKeypair], {
                commitment: "confirmed",
            });
        }
        beforeEach(async () => {
            await appendMessage(message);
        });
        it("changes the owner on the payload", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            const newOwnerPubkey = Keypair.generate().publicKey;
            await transferMessage(newOwnerPubkey, 0);
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                newOwnerPubkey,
                message,
                0
            );
            const actualRoot = await getActualRoot();
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "On-chain root hash does not equal expected hash"
            );
        });
        it("fails if the message is modified", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            const newOwnerPubkey = Keypair.generate().publicKey;
            try {
                await transferMessage(newOwnerPubkey, 0, {
                    overrides: { message: "mOdIfIeD mEsSaGe" },
                });
                assert(
                    false,
                    "Transaction should have failed since the message was modified"
                );
            } catch (e) {}
            const actualRoot = await getActualRoot();
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "The transaction should have failed because the message was " +
                "modified, but never the less, the on-chain root hash changed."
            );
        });
        it("fails if someone other than the owner tries to modify a leaf", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            const thiefKeypair = Keypair.generate();
            await Gummyroll.provider.connection.confirmTransaction(
                await Gummyroll.provider.connection.requestAirdrop(
                    thiefKeypair.publicKey,
                    2e9
                ),
                "confirmed"
            );
            try {
                await transferMessage(thiefKeypair.publicKey, 0, {
                    overrides: { signer: thiefKeypair },
                });
                assert(
                    false,
                    "Transaction should have failed since the signer was not the owner"
                );
            } catch (e) {
                assert(true);
            }
        });
    });
    describe("`Remove` instruction", () => {
        const message = "Message";
        async function removeMessage(
            index: number,
            config: { overrides?: { message?: string; signer?: Keypair } } = {}
        ) {
            const proofPubkeys = getProofOfLeaf(tree, index).map(({ node }) => ({
                pubkey: new PublicKey(node),
                isSigner: false,
                isWritable: false,
            }));
            const signer = config.overrides?.signer ?? feePayerKeypair;
            const transferIx = GummyrollCrud.instruction.remove(
                Buffer.from(tree.root, 0, 32),
                Buffer.from(config.overrides?.message ?? message),
                0,
                {
                    accounts: {
                        gummyrollProgram: Gummyroll.programId,
                        merkleRoll: merkleRollKeypair.publicKey,
                        owner: feePayerKeypair.publicKey,
                    },
                    signers: [signer],
                    remainingAccounts: proofPubkeys,
                }
            );
            const tx = new Transaction().add(transferIx);
            await GummyrollCrud.provider.send(tx, [signer], {
                commitment: "confirmed",
            });
        }
        beforeEach(async () => {
            await appendMessage(message);
        });
        it("removes the message", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            await removeMessage(0);
            recomputeRootByRemovingLeafFromTreeAtIndex(0);
            const actualRoot = await getActualRoot();
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "On-chain root hash does not equal expected hash"
            );
        });
        it("fails if the message is incorrect", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            try {
                await removeMessage(0, { overrides: { message: "iNcOrReCt mEsSaGe" } });
                assert(
                    false,
                    "Transaction should have failed since the message was wrong"
                );
            } catch (e) {}
            const actualRoot = await getActualRoot();
            const expectedRoot = tree.root;
            expect(expectedRoot.compare(actualRoot)).to.equal(
                0,
                "The transaction should have failed because the message was " +
                "wrong, but never the less, the on-chain root hash changed."
            );
        });
        it("fails if someone other than the owner tries to remove a leaf", async () => {
            recomputeRootByAddingLeafToTreeWithMessageAtIndex(
                feePayerKeypair.publicKey,
                message,
                0
            );
            const attackerKeypair = Keypair.generate();
            await Gummyroll.provider.connection.confirmTransaction(
                await Gummyroll.provider.connection.requestAirdrop(
                    attackerKeypair.publicKey,
                    2e9
                ),
                "confirmed"
            );
            try {
                await removeMessage(0, { overrides: { signer: attackerKeypair } });
                assert(
                    false,
                    "Transaction should have failed since the signer was not the owner"
                );
            } catch (e) {
                assert(true);
            }
        });
    });
});