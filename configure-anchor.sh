#!/bin/bash

function get_true_workspace_id() {
    TRUE_PROGRAM_ID=`solana-keygen pubkey $1`
}

function replace_declared_id() {
    echo "Setting declared ID in $2: $1"

    if [[ "$OSTYPE" == "darwin"* ]]
    then 
        sed -i '' 's/declare_id!(.*/declare_id!("'"$1"'");/g' $2
    else
        sed -i 's/declare_id!(.*/declare_id!("'"$1"'");/g' $2
    fi
}

function replace_workspace_id() {
    echo "Setting workspace id in $2: $1"
    
    if [[ "$OSTYPE" == "darwin"* ]]
    then 
        sed -i '' 's/'"$3"' = .*/'"$3"' = "'"$1"'"/g' $2
    else
        sed -i 's/'"$3"' = .*/'"$3"' = "'"$1"'"/g' $2
    fi
}

echo ""
echo ""

WORKSPACE_KEYPAIR="target/deploy/merkle_wallet-keypair.json"

if [[ -f "$WORKSPACE_KEYPAIR" ]]; then
    echo "Replacing keypair everwhere with address from $WORKSPACE_KEYPAIR"
else
    echo "Could not find workspace keypair from $WORKSPACE_KEYPAIR"
    echo "Please run 'anchor build' before running"
    exit 1;
fi


get_true_workspace_id $WORKSPACE_KEYPAIR
echo "True workspace program id is: $TRUE_PROGRAM_ID"

replace_declared_id $TRUE_PROGRAM_ID programs/merkle_wallet/src/lib.rs
replace_workspace_id $TRUE_PROGRAM_ID Anchor.toml merkle_wallet

WORKSPACE_KEYPAIR="target/deploy/gummyroll-keypair.json"

if [[ -f "$WORKSPACE_KEYPAIR" ]]; then
    echo "Replacing keypair everwhere with address from $WORKSPACE_KEYPAIR"
else
    echo "Could not find workspace keypair from $WORKSPACE_KEYPAIR"
    echo "Please run 'anchor build' before running"
    exit 1;
fi


get_true_workspace_id $WORKSPACE_KEYPAIR
echo "True workspace program id is: $TRUE_PROGRAM_ID"

replace_declared_id $TRUE_PROGRAM_ID programs/gummyroll/src/lib.rs
replace_workspace_id $TRUE_PROGRAM_ID Anchor.toml gummyroll

echo ""
echo ""
