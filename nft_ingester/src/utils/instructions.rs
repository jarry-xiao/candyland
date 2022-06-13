use flatbuffers::{ForwardsUOffset, Vector};
use plerkle_serialization::transaction_info_generated::transaction_info;
use plerkle_serialization::transaction_info_generated::transaction_info::TransactionInfo;

pub fn order_instructions<'a>(
    transaction_info: &TransactionInfo<'a>,
) -> Vec<(
    transaction_info::Pubkey<'a>,
    transaction_info::CompiledInstruction<'a>,
)> {
    let mut ordered_ixs: Vec<(
        transaction_info::Pubkey,
        transaction_info::CompiledInstruction,
    )> = vec![];
    // Get inner instructions.
    let inner_ix_list = transaction_info.inner_instructions();

    // Get outer instructions.
    let outer_instructions = match transaction_info.outer_instructions() {
        None => {
            println!("outer instructions deserialization error");
            return ordered_ixs;
        }
        Some(instructions) => instructions,
    };

    // Get account keys.
    let keys = match transaction_info.account_keys() {
        None => {
            println!("account_keys deserialization error");
            return ordered_ixs;
        }
        Some(keys) => keys,
    };

    for (i, instruction) in outer_instructions.iter().enumerate() {
        let program_id = keys.get(instruction.program_id_index() as usize);
        let program_id = program_id;
        ordered_ixs.push((program_id, instruction));

        if let Some(inner_ixs) = get_inner_ixs(inner_ix_list, i) {
            for inner_ix_instance in inner_ixs.instructions().unwrap() {
                let inner_program_id = keys.get(inner_ix_instance.program_id_index() as usize);
                ordered_ixs.push((inner_program_id, inner_ix_instance));
            }
        }
    }

    ordered_ixs
}

fn get_inner_ixs<'a>(
    inner_ixs: Option<Vector<'a, ForwardsUOffset<transaction_info::InnerInstructions<'_>>>>,
    outer_index: usize,
) -> Option<transaction_info::InnerInstructions<'a>> {
    match inner_ixs {
        Some(inner_ix_list) => {
            for inner_ixs in inner_ix_list {
                if inner_ixs.index() == (outer_index as u8) {
                    return Some(inner_ixs);
                }
            }
            None
        }
        None => None,
    }
}
