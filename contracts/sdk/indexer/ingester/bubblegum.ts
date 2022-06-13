import { LeafSchema } from "../../bubblegum/src/generated";
import { NewLeafEvent } from "../indexer/bubblegum";
import { ChangeLogEvent } from "../indexer/gummyroll";
import { OptionalInfo } from "../indexer/utils";

/// Ingest into database here
export function handleBubblegumMint(newLeafEvent: NewLeafEvent, leafSchemaEvent: LeafSchema, changeLog: ChangeLogEvent, optionalInfo: OptionalInfo) {

}

/// Ingest into database here
export function handleBubblegumCreateTree(changeLog: ChangeLogEvent, optionalInfo: OptionalInfo) {

}
