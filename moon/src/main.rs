use sha2::{Sha256, Digest};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;

const DATA_FILE: &str = "moon.chain";
const P2P_PORT: u16 = 38333;
const BLOCK_TIME: u64 = 60;
const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
const HALVING_INTERVAL: u64 = 210_000;
const INITIAL_REWARD: u64 = 50;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct Block {
    index: u64,
    timestamp: u64,
    prev_hash: String,
    hash: String,
    nonce: u64,
    difficulty: u32,
    txs: Vec<String>,
    coinbase: String,
}

fn load_chain() -> Vec<Block> {
    fs::read(DATA_FILE).ok().and_then(|d| bincode::deserialize(&d).ok()).unwrap_or_default()
}

fn save_chain(chain: &[Block]) {
    let data = bincode::serialize(chain).unwrap();
    let _ = fs::write(DATA_FILE, &data);
    let _ = fs::write("moon.chain.safe", &data);
}

fn adjust_difficulty(chain: &[Block]) -> u32 {
    if chain.len() as u64 % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
        return chain.last().unwrap().difficulty;
    }
    let last = chain.last().unwrap();
    let prev = &chain[chain.len() - DIFFICULTY_ADJUSTMENT_INTERVAL as usize];
    let time_expected = DIFFICULTY_ADJUSTMENT_INTERVAL * BLOCK_TIME;
    let time_taken = last.timestamp.saturating_sub(prev.timestamp);
    if time_taken < time_expected / 2 {
        last.difficulty.saturating_add(1)
    } else if time_taken > time_expected * 2 && last.difficulty > 1 {
        last.difficulty - 1
    } else {
        last.difficulty
    }
}

fn get_reward(height: u64) -> u64 {
    INITIAL_REWARD >> (height / HALVING_INTERVAL)
}

fn main() -> std::io::Result<()> {
    let coinbase = "MC7GUBTOENK3BFW5GGHIDN7R5UQ3MF37Q".to_string();
    let chain = Arc::new(Mutex::new(load_chain()));

    // MINADO + ESCRIBE ARCHIVOS PARA WINDOWS
    {
        let chain = chain.clone();
        thread::spawn(move || {
            let mut chain_local = chain.lock().unwrap().clone();
            let mut height = chain_local.len() as u64;
            let mut prev_hash = chain_local.last().map(|b| b.hash.clone()).unwrap_or("0".repeat(64));

            if chain_local.is_empty() {
                println!("Bloque Génesis creado");
                chain_local.push(Block {
                    index: 0,
                    timestamp: 1764614400,
                    prev_hash: "0".repeat(64),
                    hash: "0".repeat(64),
                    nonce: 0,
                    difficulty: 4,
                    txs: vec![],
                    coinbase: coinbase.clone(),
                });
                save_chain(&chain_local);
            }

            println!("MOON CARGADA! Bloque #{height} | Balance: {} MOON", height * 50);
            println!("Bloque cada {BLOCK_TIME} segundos | Escribiendo balance.txt y last_block.txt para Windows");

            loop {
                let difficulty = adjust_difficulty(&chain_local);
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let mut nonce = 0u64;
                let block_hash = loop {
                    let data = format!("{height}{prev_hash}{timestamp}{nonce}{difficulty}");
                    let hash = format!("{:x}", Sha256::digest(data.as_bytes()));
                    if hash.starts_with(&"0".repeat(difficulty as usize)) {
                        break hash;
                    }
                    nonce += 1;
                };

                chain_local.push(Block {
                    index: height + 1,
                    timestamp,
                    prev_hash: prev_hash.clone(),
                    hash: block_hash.clone(),
                    nonce,
                    difficulty,
                    txs: vec![],
                    coinbase: coinbase.clone(),
                });
                save_chain(&chain_local);

                // ESCRIBE ARCHIVOS PARA WINDOWS
                let _ = fs::write("balance.txt", ((height + 1) * 50).to_string());
                let _ = fs::write("last_block.txt", (height + 1).to_string());

                println!("Bloque #{} minado → +{} MOON | Balance KNKI: {} MOON", height + 1, get_reward(height), (height + 1) * 50);

                prev_hash = block_hash;
                height += 1;
                thread::sleep(Duration::from_secs(BLOCK_TIME));
            }
        });
    }

    // P2P
    {
        let chain = chain.clone();
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let listener = TcpListener::bind(format!("0.0.0.0:{P2P_PORT}")).await.unwrap();
                println!("P2P activo en puerto {P2P_PORT}");
                loop {
                    let (mut socket, addr) = listener.accept().await.unwrap();
                    let chain = chain.lock().unwrap().clone();
                    let data = bincode::serialize(&chain).unwrap();
                    let _ = socket.write_all(&data).await;
                    println!("Cadena enviada a {addr}");
                }
            });
        });
    }

    println!("MOON corriendo | 1 minuto por bloque | Sincronizado con Windows");
    loop { thread::sleep(Duration::from_secs(3600)); }
}
