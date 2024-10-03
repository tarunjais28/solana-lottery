# Contracts

We have two contracts in here:
* nezha-staking  
  This is the nezha game contract.
  You can use `nezha-staking-cli` to execute the contract instructions.
* staking
  This is NEZ token staking contract.
  You can use `nez-cli` to execute the contract instructions.

## Nezha Staking
This is the main Nezha Game contract.

## Staking
This contract provides locking facility for NEZ tokens.

Run `just nez-cli` to see the available options.  
Run `just nez-cli args..` to run the CLI.

## Local Testing Environment

* Start local validator  
`just start-local-test-env`

  This will spin up a container with solana test validator and load the program on to it.

* Destroy the docker containers and reset the state of the validator  
`just destroy-local-test-env`

## Other commands

Run `just` to see the available commands.

## Task Runner

We use `just` as our task runner. You can obtain it from here: https://github.com/casey/just
