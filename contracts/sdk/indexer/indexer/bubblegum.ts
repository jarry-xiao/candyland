import { ParsedLog, getIxName, ParserState } from "./utils";

export type BubblegumIx =
    'Redeem' | 'Decompress' | 'Transfer' | 'CreateTree' | 'Mint' | 'CancelRedeem' | 'Delegate';

export function parseBubblegum(parsedLog: ParsedLog, parser: ParserState) {
    const ixName = getIxName(parsedLog.logs[0] as string);
    console.log(ixName)
    switch (ixName) {
        case 'CreateTree':
            parseBubblegumCreateTree();
        case 'Mint':
            parseBubblegumMint();
        case 'Redeem':
            parseBubblegumRedeem();
        case 'CancelRedeem':
            parseBubblegumCancelRedeem()
        case 'Transfer':
            parseBubblegumTransfer();
        case 'Delegate':
            parseBubblegumDelegate();
    }
}

export function parseBubblegumMint() {

}

export function parseBubblegumTransfer() {

}

export function parseBubblegumCreateTree() {

}

export function parseBubblegumDelegate() {

}

export function parseBubblegumRedeem() {

}

export function parseBubblegumCancelRedeem() {

}

export function parseBubblegumDecompress() {

}
