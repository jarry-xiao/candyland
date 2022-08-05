#!/usr/bin/env bash
args=(
  --reset
  -um
  -c metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s
  -c TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
  -c ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
  --bpf-program BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY ../../target/deploy/bubblegum.so
  --bpf-program GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU ../../target/deploy/gummyroll.so
  --bpf-program GBALLoMcmimUutWvtNdFFGH5oguS7ghUUV6toQPppuTW ../../target/deploy/gumball_machine.so
  --bpf-program WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh ../../target/deploy/candy_wrapper.so
)
echo "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
