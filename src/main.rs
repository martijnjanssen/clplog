use regex::Captures;
use regex::Regex;
use std::boxed::Box;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
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

    let stdout = io::stdout();
    let stdout = stdout.lock();
    let mut buf_writer = BufWriter::new(stdout);

    let mut match_counter = 0;
    let mut no_match_counter = 0;

    // Regex matching entire line, 2 matching groups, omitting date+time, separated on semicolon
    // let re = Regex::new(r"\d{4}-(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Oct|Sep|Nov|Dec)-\d{2}\s\d{2}:\d{2}:\d{2}\.\d{9}\s(\w+):(.+)").unwrap();

    // Shorter regex separating on spaces in the log line, first match is the entire line, 1 is the origin, 2 is the level, 3 is the message
    let re = Regex::new(r".{11}\s.{18}\s(\w+):(\w+)\s(.+)").unwrap();

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                match_counter = match_counter + 1;
                let res = match_line(mtch);
                let _ = buf_writer.write_fmt(format_args!("{}\n", res.as_str()));
            }
            None => {
                no_match_counter = no_match_counter + 1;
                // eprintln!("found no match in line: {}", l);
            }
        }
    }

    println!("total number of matches: {}", match_counter);
    println!("total number of non-matches: {}", no_match_counter);

    Ok(())
}

fn match_line(mtch: Captures) -> String {
    // Match on all log categories
    let res = match mtch.get(1).unwrap().as_str() {
        "NetworkOPs" => mtch.get(3).unwrap().as_str(),
        "LedgerConsensus" => mtch.get(3).unwrap().as_str(),
        "LedgerMaster" => mtch.get(3).unwrap().as_str(),
        "Protocol" => mtch.get(3).unwrap().as_str(),
        "Peer" => mtch.get(3).unwrap().as_str(),
        "Application" => mtch.get(3).unwrap().as_str(),
        "LoadManager" => mtch.get(3).unwrap().as_str(),
        "LoadMonitor" => mtch.get(3).unwrap().as_str(),
        "PeerFinder" => mtch.get(3).unwrap().as_str(),
        "ManifestCache" => mtch.get(3).unwrap().as_str(),
        "Server" => mtch.get(3).unwrap().as_str(),
        "Validations" => mtch.get(3).unwrap().as_str(),
        "Resource" => mtch.get(3).unwrap().as_str(),
        "Ledger" => mtch.get(3).unwrap().as_str(),
        "JobQueue" => mtch.get(3).unwrap().as_str(),
        "NodeStore" => mtch.get(3).unwrap().as_str(),
        "TaggedCache" => mtch.get(3).unwrap().as_str(),
        "Amendments" => mtch.get(3).unwrap().as_str(),
        "OrderBookDB" => mtch.get(3).unwrap().as_str(),
        "ValidatorList" => mtch.get(3).unwrap().as_str(),
        "ValidatorSite" => mtch.get(3).unwrap().as_str(),
        "Flow" => mtch.get(3).unwrap().as_str(),
        "TimeKeeper" => mtch.get(3).unwrap().as_str(),
        "InboundLedger" => mtch.get(3).unwrap().as_str(),
        "TransactionAcquire" => mtch.get(3).unwrap().as_str(),
        "LedgerHistory" => mtch.get(3).unwrap().as_str(),
        unknown => {
            eprintln!("encountered unknown event \"{}\"", unknown);
            "unknown log"
        }
    };

    return String::from(res);
}
