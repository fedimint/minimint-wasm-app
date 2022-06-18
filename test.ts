import init, { UserClient } from "./pkg/minimint_wasm"
import cfg from "./src/cfg.json"


await init();
const client = new UserClient(cfg);
const peginAddr = client.peg_in_address();

alert(`pegin address: ${peginAddr}`)

const txoutProof = prompt("Enter txout proof");
const transaction = prompt("Enter transaction");
const txid = await client.peg_in(txoutProof, transaction);

alert(`Transaction id: ${txid}`)

client.free();
