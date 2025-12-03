use sha2::{Sha256, Digest};
use std::fs;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bincode;
use secp256k1::{Secp256k1, SecretKey, Message};
use secp256k1::rand::rngs::OsRng;
use std::collections::HashMap;
use std::env;

const DATA_FILE: &str = "moon.chain";
const KEY_FILE: &str = "wallet.key";
const BLOCK_TIME: u64 = 60;
const MY_ADDRESS: &str = "MC7GUBTOENK3BFW5GGHIDN7R5UQ3MF37Q";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
struct UtxoKey { tx_hash: String, index: u64 }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct TxOut { to: String, amount: u64 }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct TxIn { prev_tx_hash: String, prev_out_index: u64, signature: Vec<u8> }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct Tx { inputs: Vec<TxIn>, outputs: Vec<TxOut> }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct Block {
    index: u64, timestamp: u64, prev_hash: String, hash: String,
    nonce: u64, difficulty: u32, txs: Vec<Tx>, coinbase: TxOut,
}

fn load_chain() -> Vec<Block> {
    fs::read(DATA_FILE).ok().and_then(|d| bincode::deserialize(&d).ok()).unwrap_or_default()
}

fn save_chain(chain: &[Block]) {
    let data = bincode::serialize(chain).unwrap();
    let _ = fs::write(DATA_FILE, &data);
    let _ = fs::write("moon.chain.safe", &data);
}

fn get_utxos(chain: &[Block], address: &str) -> HashMap<UtxoKey, TxOut> {
    let mut utxos = HashMap::new();
    let mut spent = HashMap::new();

    for block in chain {
        let key = UtxoKey { tx_hash: block.hash.clone(), index: 0 };
        if block.coinbase.to == address {
            utxos.insert(key, block.coinbase.clone());
        }

        for tx in &block.txs {
            let tx_hash = format!("{:x}", Sha256::digest(&bincode::serialize(tx).unwrap()));
            for (i, output) in tx.outputs.iter().enumerate() {
                let key = UtxoKey { tx_hash: tx_hash.clone(), index: i as u64 };
                if output.to == address {
                    utxos.insert(key, output.clone());
                }
            }
            for input in &tx.inputs {
                spent.insert(UtxoKey { tx_hash: input.prev_tx_hash.clone(), index: input.prev_out_index }, true);
            }
        }
    }
    utxos.retain(|k, _| !spent.contains_key(k));
    utxos
}

fn get_balance(chain: &[Block], address: &str) -> u64 {
    get_utxos(chain, address).values().map(|o| o.amount).sum()
}

fn main() {
    let secp = Secp256k1::new();
    let secret_key = if fs::metadata(KEY_FILE).is_ok() {
        let data = fs::read(KEY_FILE).unwrap();
        SecretKey::from_slice(&data).expect("Clave inválida")
    } else {
        let (sk, _) = secp.generate_keypair(&mut OsRng);
        fs::write(KEY_FILE, sk.secret_bytes()).unwrap();
        println!("NUEVA WALLET CREADA – CLAVE PRIVADA GUARDADA EN wallet.key");
        sk
    };

    println!("MOON v3.0 – CADENA CON UTXO REAL");
    println!("TU DIRECCIÓN: {MY_ADDRESS}");

    let mut chain = load_chain();
    let mut height = chain.len() as u64;
    let mut prev_hash = chain.last().map(|b| b.hash.clone()).unwrap_or("0".repeat(64));

    if chain.is_empty() {
        println!("Bloque Génesis creado");
        chain.push(Block {
            index: 0, timestamp: 1764614400, prev_hash: "0".repeat(64), hash: "0".repeat(64),
            nonce: 0, difficulty: 4, txs: vec![],
            coinbase: TxOut { to: MY_ADDRESS.to_string(), amount: 50 },
        });
        save_chain(&chain);
    }

    // COMANDO ENVIAR (resta del balance)
    let args: Vec<String> = env::args().collect();
    if args.len() == 4 && args[1] == "send" {
        let to = args[2].clone();
        let amount: u64 = args[3].parse().unwrap();

        let utxos = get_utxos(&chain, MY_ADDRESS);
        let mut total = 0u64;
        let mut inputs = vec![];

        for (key, utxo) in &utxos {
            if total >= amount { break; }
            total += utxo.amount;
            inputs.push(TxIn {
                prev_tx_hash: key.tx_hash.clone(),
                prev_out_index: key.index,
                signature: vec![],
            });
        }

        if total < amount {
            println!("ERROR: Fondos insuficientes");
            return;
        }

        let change = total - amount;
        let mut outputs = vec![TxOut { to: to.clone(), amount }];
        if change > 0 {
            outputs.push(TxOut { to: MY_ADDRESS.to_string(), amount: change });
        }

        let tx = Tx { inputs, outputs };
        let tx_data = bincode::serialize(&tx).unwrap();
        let message = Message::from_digest_slice(&Sha256::digest(&tx_data)).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret_key);

        let _ = fs::write("pending_tx.bin", bincode::serialize(&tx).unwrap());
        println!("ENVIANDO {amount} MOON A {to} – SE DESCONTARÁ DE TU BALANCE");
        return;
    }

    // MINADO (incluye transacciones pendientes)
    loop {
        let mut pending_txs = vec![];
        if let Ok(data) = fs::read("pending_tx.bin") {
            if let Ok(tx) = bincode::deserialize::<Tx>(&data) {
                pending_txs.push(tx);
                println!("ENVÍO INCLUIDO – TU BALANCE BAJÓ");
                fs::remove_file("pending_tx.bin").ok();
            }
        }

        let difficulty = 4;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut nonce = 0u64;
        let block_hash = loop {
            let data = format!("{height}{prev_hash}{timestamp}{nonce}{difficulty}");
            let hash = format!("{:x}", Sha256::digest(data.as_bytes()));
            if hash.starts_with(&"0".repeat(difficulty as usize)) { break hash; }
            nonce += 1;
        };

        chain.push(Block {
            index: height + 1, timestamp, prev_hash: prev_hash.clone(),
            hash: block_hash.clone(), nonce, difficulty, txs: pending_txs,
            coinbase: TxOut { to: MY_ADDRESS.to_string(), amount: 50 },
        });
        save_chain(&chain);

        let my_balance = get_balance(&chain, MY_ADDRESS);
        let current_height = height + 1;

        // PARA WINDOWS EN VIVO
        let _ = fs::write("/mnt/c/temp_moon/balance.txt", my_balance.to_string());
        let _ = fs::write("/mnt/c/temp_moon/height.txt", current_height.to_string());

        println!("Height: {current_height} Balance: {my_balance} MOON Hash: {block_hash}");

        prev_hash = block_hash;
        height += 1;
        thread::sleep(Duration::from_secs(BLOCK_TIME));
    }
}
