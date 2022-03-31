use crate::error::PlerkleError;
use lazy_static::lazy_static;
use regex::Regex;
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, ReplicaTransactionInfo,
};



pub fn handle_change_log_event(
    transaction: &ReplicaTransactionInfo,
) -> Result<Vec<String>, GeyserPluginError> {
    lazy_static! {
        static ref CLRE: Regex = Regex::new(
            r"Program log: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)"
        )
        .unwrap();
    }
    let mut events: Vec<String> = vec![];
    let err = Err(GeyserPluginError::Custom(Box::new(
        PlerkleError::EventError {},
    )));
    match transaction.transaction_status_meta.log_messages.as_ref() {
        Some(lines) => {
            for line in lines {
                let captures = CLRE.captures(line);
                let b64raw = captures.and_then(|c| c.get(1)).map(|c| c.as_str());
                b64raw.inspect(|raw| events.push((**raw).parse().unwrap()));
            }
            if events.is_empty() {
                err
            } else {
                Ok(events)
            }
        }
        None => err,
    }
}
