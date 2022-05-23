use flatbuffers::{ForwardsUOffset, Vector};
use lazy_static::lazy_static;
use solana_sdk::pubkey::Pubkey;
use regex::Regex;

pub fn filter_events_from_logs(log_messages: &Vec<&str>) -> Result<Vec<String>, ()> {
    lazy_static! {
        static ref CLRE: Regex = Regex::new(
            r"Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)"
        )
        .unwrap();
    }
    let mut events: Vec<String> = vec![];

    for line in log_messages {
        let line_str = String::from(*line);
        let captures = CLRE.captures(&line_str);
        let b64raw = captures.and_then(|c| c.get(1)).map(|c| c.as_str());
        b64raw.map(|raw| events.push((raw).parse().unwrap()));
    }
    if events.is_empty() {
        println!("No events captured!");
        Err(())
    } else {
        Ok(events)
    }
}

pub fn parse_logs<'a>(
    log_messages: Option<Vector<'a, ForwardsUOffset<&str>>>,
) -> Result<Vec<(Pubkey, Vec<&'a str>)>, ()> {
    lazy_static! {
        static ref PLRE: Regex = Regex::new(r"Program (\w*) invoke \[(\d)\]").unwrap();
    }
    let mut program_logs: Vec<(Pubkey, Vec<&str>)> = vec![];

    match log_messages {
        Some(logs) => {
            for log in logs {
                let captures = PLRE.captures(log);
                let pubkey_bytes = captures
                    .and_then(|c| c.get(1))
                    .map(|c| bs58::decode(&c.as_str()).into_vec().unwrap());

                match pubkey_bytes {
                    None => {
                        let last_program_log = program_logs.last_mut().unwrap();
                        (*last_program_log).1.push(log);
                    }
                    Some(bytes) => {
                        program_logs.push((Pubkey::new(&bytes), vec![]));
                    }
                }
            }
            Ok(program_logs)
        }
        None => {
            println!("No logs found in transaction info!");
            Err(())
        }
    }
}
