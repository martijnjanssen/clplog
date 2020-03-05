use regex::Regex;
use std::boxed::Box;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;
use std::result::Result;

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

    let mut counter = 0;
    let mut no_match_counter = 0;

    // Regex matching entire line, 2 matching groups, omitting date+time, separated on semicolon
    // let re = Regex::new(r"\d{4}-(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Oct|Sep|Nov|Dec)-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{9}\s(\w+):(.+)").unwrap();
    // Shorter regex separating on spaces in the log line, matching on the text around the last ":"
    let re = Regex::new(r".{11}\s.{18}\s(\w+):(.+)").unwrap();

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                counter = counter + 1;

                // Match on all log categories
                match mtch.get(1).unwrap().as_str() {
                    "NetworkOPs" => {}
                    "LedgerConsensus" => {}
                    "LedgerMaster" => {}
                    "Protocol" => {}
                    "Peer" => {}
                    "Application" => {}
                    "LoadManager" => {}
                    "LoadMonitor" => {}
                    "PeerFinder" => {}
                    "ManifestCache" => {}
                    "Server" => {}
                    "Validations" => {}
                    "Resource" => {}
                    "Ledger" => {}
                    "JobQueue" => {}
                    "NodeStore" => {}
                    "TaggedCache" => {}
                    "Amendments" => {}
                    "OrderBookDB" => {}
                    "ValidatorList" => {}
                    "ValidatorSite" => {}
                    "Flow" => {}
                    "TimeKeeper" => {}
                    "InboundLedger" => {}
                    "TransactionAcquire" => {}
                    "LedgerHistory" => {}
                    unknown => println!("encountered unknown event \"{}\"", unknown),
                }

                // 0 index is the entire match, 1 and 2 are the first 2 capturing groups
                // format!(
                //     "{}: {}",
                //     mtch.get(1).unwrap().as_str(),
                //     mtch.get(2).unwrap().as_str()
                // );
            }
            // None => eprintln!("found no match in line: {}", l),
            None => no_match_counter = no_match_counter + 1,
        }
    }
    println!("total number of matches: {}", counter);
    println!("total number of non-matches: {}", no_match_counter);
    Ok(())
}
