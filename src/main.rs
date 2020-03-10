use regex::Regex;
use std::boxed::Box;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;
use std::result::Result;

static LOG_ENTERING_CONSENSUS: &str = "LedgerConsensus:NFO Entering consensus process";

fn main() {
    if let Err(error) = try_main() {
        eprintln!("{}", error);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
            "missing argument for logfile",
        ));
    }

    // First argument is the command itself
    let filename: &String = &args[1];

    let file = File::open(filename)?;
    let buf_reader = BufReader::new(file);
    let mut contents = buf_reader.lines();

    // let stdout = io::stdout();
    // let stdout = stdout.lock();
    // let mut buf_writer = BufWriter::new(stdout);

    let mut match_counter = 0;
    let mut no_match_counter = 0;

    // Count distinct logs
    let mut log_id_counter = 0;
    // Map log_string -> log_id
    let mut log_id_map: HashMap<String, u64> = HashMap::new();
    let mut all_log_sequence = Vec::<Vec<u64>>::new();
    let mut log_list = Vec::<String>::new();
    let mut log_counts = Vec::<u64>::new();

    // Regex matching entire line, 2 matching groups, omitting date+time, separated on semicolon
    // let re = Regex::new(r"\d{4}-(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Oct|Sep|Nov|Dec)-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{9}\s(\w+):(.+)").unwrap();

    // Shorter regex separating on spaces in the log line, first match is the entire line, 1 is the origin, 2 is the level, 3 is the message
    let re = Regex::new(r".{11}\s.{18}\s(\w+:\w+\s.+)").unwrap();

    let re_base_16 = Regex::new(r"[0-9A-F]{64}").unwrap();
    let re_alpha_num_id = Regex::new(r"[A-Za-z0-9]{52}").unwrap();
    let re_ip = Regex::new(r"(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?").unwrap();
    let re_hash_num = Regex::new(r"#\d+").unwrap();

    let mut started = false;

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                match_counter += 1;

                let msg = match mtch.get(1) {
                    Some(m) => m.as_str(),
                    None => {
                        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "unable to get match from parsed log",
                        ))
                    }
                };

                if msg.starts_with(LOG_ENTERING_CONSENSUS) {
                    all_log_sequence.push(Vec::new());
                    started = true;
                }

                if !started {
                    continue;
                }

                if msg.starts_with("Application:NFO") || msg.starts_with("Peer:") {
                    continue;
                }

                // replace base-16 hashes of length 64 (e.g.: 58B57FBEF009EB802DA44B7B35E362DA33648FCD2FE3C3DA235C54EFC8A082A8)
                let msg_sanitized = &re_base_16.replace_all(msg, "some-base-16-hash");
                // replace alpha numerical ids of length 52 (e.g.: nHBe4vqSAzjpPRLKwSFzRFtmvzXaf5wPPmuVrQCAoJoS1zskgDA4)
                let msg_sanitized = &re_alpha_num_id.replace_all(msg_sanitized, "some-id");
                // replace ip addresses
                let msg_sanitized = &re_ip.replace_all(msg_sanitized, "some-ip");
                // replace numbers with '#' prefix (e.g.: #5334)
                let msg_sanitized = re_hash_num
                    .replace_all(msg_sanitized, "#some-num")
                    .to_string();

                // if this is a new log
                if !log_id_map.contains_key(&msg_sanitized) {
                    let msg_sanitized_clone_1 = msg_sanitized.clone();
                    let msg_sanitized_clone_2 = msg_sanitized.clone();
                    // add it to the map
                    log_id_map.insert(msg_sanitized_clone_1, log_id_counter);
                    // add to the list
                    log_list.push(msg_sanitized_clone_2);
                    // initialize log counts as zero
                    log_counts.push(0);
                    // increase unique log counter
                    log_id_counter += 1;
                }

                // get the log id
                let log_id = match log_id_map.get(&msg_sanitized) {
                    Some(id) => id,
                    None => {
                        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "should have entry for log id",
                        ));
                    }
                };

                // increase the count
                match log_counts.get_mut(*log_id as usize) {
                    Some(count) => *count += 1,
                    None => {
                        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "should have count for entry",
                        ));
                    }
                }

                // append the id to the current sequence, if none found, add a new one
                let log_index = all_log_sequence.len() - 1;
                match all_log_sequence.get_mut(log_index) {
                    Some(sequence) => sequence.push(*log_id),
                    None => {
                        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                            "should have entry in log sequence",
                        ));
                    }
                }
            }
            None => {
                no_match_counter += 1;
                // eprintln!("found no match in line: {}", l);
            }
        }
    }

    // dbg!(all_log_sequence);
    // dbg!(log_list);

    println!("{} {}", all_log_sequence.len(), log_list.len());
    for (pos, item) in all_log_sequence.iter().enumerate() {
        print!("1 {}", item.len());
        for log_id in item.iter() {
            print!(" {}", log_id)
        }
        println!()
    }

    // println!("total number of matches: {}", match_counter);
    // println!("total number of non-matches: {}", no_match_counter);

    Ok(())
}

// fn match_line(mtch: Captures) -> String {
//     // Match on all log categories
//     let res = match mtch.get(1).unwrap().as_str() {
//         "NetworkOPs" => mtch.get(3).unwrap().as_str(),
//         "LedgerConsensus" => mtch.get(3).unwrap().as_str(),
//         "LedgerMaster" => mtch.get(3).unwrap().as_str(),
//         "Protocol" => mtch.get(3).unwrap().as_str(),
//         "Peer" => mtch.get(3).unwrap().as_str(),
//         "Application" => mtch.get(3).unwrap().as_str(),
//         "LoadManager" => mtch.get(3).unwrap().as_str(),
//         "LoadMonitor" => mtch.get(3).unwrap().as_str(),
//         "PeerFinder" => mtch.get(3).unwrap().as_str(),
//         "ManifestCache" => mtch.get(3).unwrap().as_str(),
//         "Server" => mtch.get(3).unwrap().as_str(),
//         "Validations" => mtch.get(3).unwrap().as_str(),
//         "Resource" => mtch.get(3).unwrap().as_str(),
//         "Ledger" => mtch.get(3).unwrap().as_str(),
//         "JobQueue" => mtch.get(3).unwrap().as_str(),
//         "NodeStore" => mtch.get(3).unwrap().as_str(),
//         "TaggedCache" => mtch.get(3).unwrap().as_str(),
//         "Amendments" => mtch.get(3).unwrap().as_str(),
//         "OrderBookDB" => mtch.get(3).unwrap().as_str(),
//         "ValidatorList" => mtch.get(3).unwrap().as_str(),
//         "ValidatorSite" => mtch.get(3).unwrap().as_str(),
//         "Flow" => mtch.get(3).unwrap().as_str(),
//         "TimeKeeper" => mtch.get(3).unwrap().as_str(),
//         "InboundLedger" => mtch.get(3).unwrap().as_str(),
//         "TransactionAcquire" => mtch.get(3).unwrap().as_str(),
//         "LedgerHistory" => mtch.get(3).unwrap().as_str(),
//         unknown => {
//             eprintln!("encountered unknown event \"{}\"", unknown);
//             "unknown log"
//         }
//     };

//     return String::from(res);
// }
