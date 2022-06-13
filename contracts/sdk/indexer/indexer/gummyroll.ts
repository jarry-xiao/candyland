import * as anchor from '@project-serum/anchor';
import { Gummyroll } from "../../gummyroll"
import { parseEventFromLog } from "./utils"

export function parseGummyrollAppend(logs: string[], gummyroll: anchor.Program<Gummyroll>) {
    let newLeafEvent = parseEventFromLog(logs[1], gummyroll.idl);
    console.log(logs[1], logs[-2]);
    let changeLogEvent = parseEventFromLog(logs[logs.length - 2], gummyroll.idl);
    console.log(newLeafEvent, changeLogEvent);
}
