use bs58;

fn main() {
    let key = "4TtAh6N9pq4BjEhwiC2o2daZXqkSVyEgRrdV27wcusSoRzFtopTcjT2APnnuzFf2sDR2RL3fbFbEUov8DgvBKJWD";
    
    match bs58::decode(key).into_vec() {
        Ok(decoded) => {
            println!("Key length: {} bytes", decoded.len());
            println!("First 10 bytes: {:?}", &decoded[..std::cmp::min(10, decoded.len())]);
            if decoded.len() >= 32 {
                println!("Last 10 bytes: {:?}", &decoded[decoded.len()-10..]);
            }
        },
        Err(e) => println!("Failed to decode: {}", e),
    }
}
