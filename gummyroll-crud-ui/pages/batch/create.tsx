import React, { useState, SyntheticEvent } from 'react';
import { NextPage } from "next";
import { useRouter } from "next/router";
import { useAnchorWallet } from '@solana/wallet-adapter-react';
import { useSWRConfig } from 'swr';
import Button from '../../components/Button';
import { parse } from 'csv-parse/sync';

const BatchMintTree: NextPage = () => {
    const router = useRouter();
    const dataRef = React.createRef<HTMLTextAreaElement>();
    const { mutate } = useSWRConfig();
    const anchorWallet = useAnchorWallet();
    if (!anchorWallet) {
        throw new Error("You must be logged in to create a new asset.");
    }
    const [changelogId, setChangelogId] = useState("");
    const [metadataId, setMetadataId] = useState("");
    const [isLoading, setIsLoading] = useState(false);

    async function handleBatchMintTree(
        e: SyntheticEvent<HTMLButtonElement, MouseEvent>
    ) {
        e.preventDefault();
        console.log(changelogId);
        console.log(metadataId);

        const changeLogUri = `https://arweave.net/${changelogId}`;
        const metadataUri = `https://arweave.net/${metadataId}`;
        setIsLoading(true);
        let metadataText = await fetch(metadataUri, {
            redirect: 'follow',
        }).then((resp) => resp.text());
        let metadataMessages = parse(
            metadataText,
            {
                columns: true,
                skipEmptyLines: true,
            });

        setIsLoading(false);
        console.log(metadataMessages);

        // await mutate(async (currentData) => {
        // const account = await createTree(anchorWallet, 14, 64);
        // newTree = {
        //     account: account.toBase58(),
        //     authority: anchorWallet.publicKey.toBase58(),
        // };
        // return [newTree, ...(currentData ?? [])];
        // });
    }

    return <div>
        {isLoading ? `Loading CSV from https://arweave.net/${changelogId}` : null}
        <div>
            <input style={{ marginRight: '5px' }} type={'text'} placeholder={"Changelog db id"} onChange={(e) => setChangelogId(e.target.value)} />
            <input style={{ marginRight: '5px' }} type={'text'} placeholder={"Metadata db id"} onChange={(e) => setMetadataId(e.target.value)} />
            <Button onClick={handleBatchMintTree}>Batch mint tree</Button>
        </div>
    </div>
}

export default BatchMintTree;
