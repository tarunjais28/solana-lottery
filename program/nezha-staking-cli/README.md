# Contract Crank Demo
## Setup
- (optional) change the staking contract address if needed in `STAKING_CONTRACT_STR` of `./src/main.rs`
- (optional) change the 'USDC' mint address if needed in `USDC_MINT_STR` of `./src/main.rs`
- build: `cargo build`
- alias: `alias demo=../../../target/debug/contract-demo`
- epoch: `export EPOCH=<epoch index>`, this epoch index needs to be new for the CLI to work
- private keys used by the demo are stored in `./keys`

## CLI options
- `demo approve-deposit <user wallet>`: approve attempted deposit of `<user wallet>`  
- `demo create-epoch $EPOCH`: creating a new epoch. If the epoch index already exists, this would fail. For this demo, the epoch end timestamp is hardcoded at creation timestamp + 60s
- `demo create-winning-combination $EPOCH`: creating the winning combination for this epoch
- `demo publish-winners $EPOCH <tier> <winning-size>`: publish winners and its winning size for this epoch for a certain tier. In demo, the winner is hard coded to be the 'demo user' `usdizAmv6mNTjiX3qdw7XydqPhh2wJHNKBaq5aJ34p2`

### Account decoder
You can decode the various accounts associated with staking contract, the respective account addresses should show up in the console if you run the above commands:
- `demo decode stake <user wallet>`: decode the stake account, which shows how much the user has staked in total
- `demo decode epoch <epoch account>`: decode the epoch account, showing epoch index and the epoch end unix timestamp value
- `demo decode winning-combination <winning combination account>`: decode the winning combination account, showing the epoch pubkey and the 6 winning numbers
- `demo decode winners <winners account>`: decode the winners account, showing who wins and whether the prize has been claim or not
