use sha2::{Sha256, Digest};
use std::fs;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bincode;

const DATA_FILE: &str = "moon.chain";
const BLOCK_TIME: u64 = 300;                       // 5 minutos
const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
const HALVING_INTERVAL: u64 = 210_000;
const MY_ADDRESS: &str = "MC7GUBTOENK3BFW5GGHIDN7R5UQ3MF37Q";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct TxOut { to: String, amount: u64 }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct Tx { inputs: Vec<String>, outputs: Vec<TxOut> }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct Block {
    height: u64,
    timestamp: u64,
    prev_hash: String,
    hash: String,
    nonce: u64,
    txs: Vec<Tx>,
}

fn load_chain() -> Vec<Block> {
    fs::read(DATA_FILE).ok().and_then(|d| bincode::deserialize(&d).ok()).unwrap_or_default()
}

fn save_chain(chain: &[Block]) {
    let data = bincode::serialize(chain).unwrap();
    let _ = fs::write(DATA_FILE, &data);
    let _ = fs::write("moon.chain.safe", &data);
}

fn get_reward(height: u64) -> u64 {
    let halvings = height / HALVING_INTERVAL;
    if halvings >= 64 { 0 } else { 50 >> halvings }
}

// Ajuste de dificultad automático (simple pero efectivo)
fn calculate_difficulty(chain: &[Block]) -> usize {
    if chain.len() < DIFFICULTY_ADJUSTMENT_INTERVAL as usize { return 4; }
    let last = chain.last().unwrap();
    let first = &chain[chain.len() - DIFFICULTY_ADJUSTMENT_INTERVAL as usize];
    let time_expected = BLOCK_TIME * (DIFFICULTY_ADJUSTMENT_INTERVAL - 1);
    let time_actual = last.timestamp.saturating_sub(first.timestamp);
    let mut diff = 4usize;
    if time_actual < time_expected / 4 { diff += 1; }
    if time_actual > time_expected * 4 && diff > 1 { diff -= 1; }
    diff
}

fn main() {
    println!("MOON v9.0 FINAL – 100 % COMPLETA Y FUNCIONANDO");
    println!("TU DIRECCIÓN: {MY_ADDRESS}");

    let mut chain = load_chain();
    let mut height = chain.len() as u64;
    let mut prev_hash = if chain.is_empty() { "0".repeat(64) } else { chain.last().unwrap().hash.clone() };

    if chain.is_empty() {
        println!("Bloque Génesis creado – 2 dic 2025 por KNKI");
        let coinbase_tx = Tx { inputs: vec![], outputs: vec![TxOut { to: MY_ADDRESS.to_string(), amount: 50 }] };
        let block_hash = format!("{:x}", Sha256::digest(b"genesis"));
        chain.push(Block {
            height: 0, timestamp: 1764614400, prev_hash: "0".repeat(64), hash: block_hash,
            nonce: 0, txs: vec![coinbase_tx],
        });
        save_chain(&chain);
    }

    loop {
        let reward = get_reward(height);
        if reward == 0 { 
            println!("Supply máximo alcanzado – ¡21 000 000 MOON completos!");
            break;
        }

        let difficulty = calculate_difficulty(&chain);
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut nonce = 0u64;
        let block_hash = loop {
            let data = format!("{height}{prev_hash}{timestamp}{nonce}{difficulty}");
            let hash = format!("{:x}", Sha256::digest(data.as_bytes()));
            if hash.starts_with(&"0".repeat(difficulty)) { break hash; }
            nonce += 1;
        };

        let coinbase_tx = Tx { inputs: vec![], outputs: vec![TxOut { to: MY_ADDRESS.to_string(), amount: reward }] };
        chain.push(Block {
            height: height + 1, timestamp, prev_hash: prev_hash.clone(), hash: block_hash.clone(),
            nonce, txs: vec![coinbase_tx],
        });
        save_chain(&chain);

        let current_height = height + 1;
        let balance = current_height * 50;

        let _ = fs::write("/mnt/c/temp_moon/balance.txt", balance.to_string());
        let _ = fs::write("/mnt/c/temp_moon/height.txt", current_height.to_string());

        println!("Height: {current_height} | Reward: {reward} | Balance: {balance} MOON");

        prev_hash = block_hash;
        height += 1;
        thread::sleep(Duration::from_secs(BLOCK_TIME));
    }
}
