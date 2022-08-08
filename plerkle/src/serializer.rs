
use {
    flatbuffers::FlatBufferBuilder,
    plerkle_serialization::{
        account_info_generated::account_info::{AccountInfo, AccountInfoArgs},
        block_info_generated,
        slot_status_info_generated::slot_status_info::{self, SlotStatusInfo, SlotStatusInfoArgs},
        transaction_info_generated::transaction_info::{
            self, TransactionInfo, TransactionInfoArgs,
        },
    },
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        ReplicaBlockInfo, SlotStatus,
        ReplicaAccountInfoV2, ReplicaTransactionInfoV2
    },
    solana_runtime::bank::RewardType,
    chrono::{DateTime, Utc}
};

pub fn serialize_account<'a>(
    mut builder: FlatBufferBuilder<'a>,
    account: &ReplicaAccountInfoV2,
    slot: u64,
    is_startup: bool,
) -> FlatBufferBuilder<'a> {
    // Serialize vector data.
    let pubkey = builder.create_vector(account.pubkey);
    let owner = builder.create_vector(account.owner);
    let data = builder.create_vector(account.data);

    // Serialize everything into Account Info table.
    let account_info = AccountInfo::create(
        &mut builder,
        &AccountInfoArgs {
            pubkey: Some(pubkey),
            lamports: account.lamports,
            owner: Some(owner),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
            data: Some(data),
            write_version: account.write_version,
            slot,
            is_startup,
        },
    );

    // Finalize buffer and return to caller.
    builder.finish(account_info, None);
    builder
}

pub fn serialize_slot_status<'a>(
    mut builder: FlatBufferBuilder<'a>,
    slot: u64,
    parent: Option<u64>,
    status: SlotStatus,
) -> FlatBufferBuilder<'a> {
    // Convert to flatbuffer enum.
    let status = match status {
        SlotStatus::Confirmed => slot_status_info::Status::Confirmed,
        SlotStatus::Processed => slot_status_info::Status::Processed,
        SlotStatus::Rooted => slot_status_info::Status::Rooted,
    };

    // Serialize everything into Slot Status Info table.
    let slot_status = SlotStatusInfo::create(
        &mut builder,
        &SlotStatusInfoArgs {
            slot,
            parent,
            status,
        },
    );

    // Finalize buffer and return to caller.
    builder.finish(slot_status, None);
    builder
}

pub fn serialize_transaction<'a>(
    mut builder: FlatBufferBuilder<'a>,
    transaction_info: &ReplicaTransactionInfoV2,
    slot: u64,
) -> FlatBufferBuilder<'a> {
    // Flatten and serialize account keys.
    let account_keys = transaction_info.transaction.message().account_keys();
    let account_keys_len = account_keys.len();

    let account_keys = if account_keys_len > 0 {
        let mut account_keys_fb_vec = Vec::with_capacity(account_keys_len);
        for key in account_keys.iter() {
            let key = builder.create_vector(&key.to_bytes());
            let pubkey = transaction_info::Pubkey::create(
                &mut builder,
                &transaction_info::PubkeyArgs { key: Some(key) },
            );
            account_keys_fb_vec.push(pubkey);
        }
        Some(builder.create_vector(&account_keys_fb_vec))
    } else {
        None
    };

    // Serialize log messages.
    let log_messages = if let Some(log_messages) = transaction_info
        .transaction_status_meta
        .log_messages
        .as_ref()
    {
        let mut log_messages_fb_vec = Vec::with_capacity(log_messages.len());
        for message in log_messages {
            log_messages_fb_vec.push(builder.create_string(&message));
        }
        Some(builder.create_vector(&log_messages_fb_vec))
    } else {
        None
    };

    // Serialize inner instructions.
    let inner_instructions = if let Some(inner_instructions_vec) = transaction_info
        .transaction_status_meta
        .inner_instructions
        .as_ref()
    {
        let mut overall_fb_vec = Vec::with_capacity(inner_instructions_vec.len());
        for inner_instructions in inner_instructions_vec.iter() {
            let index = inner_instructions.index;
            let mut instructions_fb_vec = Vec::with_capacity(inner_instructions.instructions.len());
            for compiled_instruction in inner_instructions.instructions.iter() {
                let program_id_index = compiled_instruction.program_id_index;
                let accounts = Some(builder.create_vector(&compiled_instruction.accounts));
                let data = Some(builder.create_vector(&compiled_instruction.data));
                instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                    &mut builder,
                    &transaction_info::CompiledInstructionArgs {
                        program_id_index,
                        accounts,
                        data,
                    },
                ));
            }

            let instructions = Some(builder.create_vector(&instructions_fb_vec));
            overall_fb_vec.push(transaction_info::InnerInstructions::create(
                &mut builder,
                &transaction_info::InnerInstructionsArgs {
                    index,
                    instructions,
                },
            ))
        }

        Some(builder.create_vector(&overall_fb_vec))
    } else {
        None
    };

    // Serialize outer instructions.
    let outer_instructions = transaction_info.transaction.message().instructions();
    let outer_instructions = if outer_instructions.len() > 0 {
        let mut instructions_fb_vec = Vec::with_capacity(outer_instructions.len());
        for compiled_instruction in outer_instructions.iter() {
            let program_id_index = compiled_instruction.program_id_index;
            let accounts = Some(builder.create_vector(&compiled_instruction.accounts));
            let data = Some(builder.create_vector(&compiled_instruction.data));
            instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                &mut builder,
                &transaction_info::CompiledInstructionArgs {
                    program_id_index,
                    accounts,
                    data,
                },
            ));
        }
        Some(builder.create_vector(&instructions_fb_vec))
    } else {
        None
    };
    let slot_idx = builder.create_string(&format!("{}-{}", slot, transaction_info.index));
    let seen_at = Utc::now();
    // Serialize everything into Transaction Info table.
    let transaction_info_ser = TransactionInfo::create(
        &mut builder,
        &TransactionInfoArgs {
            is_vote: transaction_info.is_vote,
            account_keys,
            log_messages,
            inner_instructions,
            outer_instructions,
            slot,
            slot_index: Some(slot_idx),
            seen_at: seen_at.timestamp_millis()
        },
    );

    // Finalize buffer and return to caller.
    builder.finish(transaction_info_ser, None);
    builder
}

pub fn serialize_block<'a>(
    mut builder: FlatBufferBuilder<'a>,
    block_info: &ReplicaBlockInfo,
) -> FlatBufferBuilder<'a> {
    // Serialize blockash.
    let blockhash = Some(builder.create_string(&block_info.blockhash));

    // Serialize rewards.
    let rewards = if block_info.rewards.len() > 0 {
        let mut rewards_fb_vec = Vec::with_capacity(block_info.rewards.len());
        for reward in block_info.rewards.iter() {
            let pubkey = Some(builder.create_vector(reward.pubkey.as_bytes()));
            let lamports = reward.lamports;
            let post_balance = reward.post_balance;

            let reward_type = if let Some(reward) = reward.reward_type {
                match reward {
                    RewardType::Fee => Some(block_info_generated::block_info::RewardType::Fee),
                    RewardType::Rent => Some(block_info_generated::block_info::RewardType::Rent),
                    RewardType::Staking => {
                        Some(block_info_generated::block_info::RewardType::Staking)
                    }
                    RewardType::Voting => {
                        Some(block_info_generated::block_info::RewardType::Voting)
                    }
                }
            } else {
                None
            };

            let commission = reward.commission;

            rewards_fb_vec.push(block_info_generated::block_info::Reward::create(
                &mut builder,
                &block_info_generated::block_info::RewardArgs {
                    pubkey,
                    lamports,
                    post_balance,
                    reward_type,
                    commission,
                },
            ));
        }
        Some(builder.create_vector(&rewards_fb_vec))
    } else {
        None
    };

    // Serialize everything into Block Info table.
    let block_info = block_info_generated::block_info::BlockInfo::create(
        &mut builder,
        &block_info_generated::block_info::BlockInfoArgs {
            slot: block_info.slot,
            blockhash,
            rewards,
            block_time: block_info.block_time,
            block_height: block_info.block_height,
        },
    );

    // Finalize buffer and return to caller.
    builder.finish(block_info, None);
    builder
}
