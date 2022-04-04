use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;

pub struct ProgramParserManager {
    registered_parsers: HashMap<Pubkey, ProgramParser>
}

impl ProgramParserManager {
    pub fn match_program() -> Result<ProgramParser> {

    }
}

pub trait ProgramParser {
    fn id(&self) -> Pubkey;
}

pub trait ProgramLogParser {

}

pub trait ProgramInstructionParser {
    
    fn parse_instructions(&self, Vec<Inst>) {
    
    }

}


pub struct GummyRollCrudParser {
    id: Pubkey
}

impl ProgramParser for GummyRollCrudParser {
    fn id(&self) -> Pubkey{
        self.id
    }
}

impl ProgramInstructionParser for GummyRollCrudParser {
    
}