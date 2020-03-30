use regex::Match;
use regex::Regex;
use std::boxed::Box;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::process;
use std::result::Result;

static LOG_ENTERING_CONSENSUS: &str = "LedgerConsensus:NFO Entering consensus process";
// Stop after number rounds
// static STOP_ROUNDS: i32 = 1000000;
// Process entire file
static STOP_ROUNDS: i32 = -1;

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

    let mut rounds = 0;

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
    let re = Regex::new(r".{11}\s.{18}\s((\w+):(\w+\s.+))").unwrap();

    let re_base_16 = Regex::new(r"[0-9A-F]{64}").unwrap();
    let re_alpha_num_id = Regex::new(r"[A-Za-z0-9]{52}").unwrap();
    let re_ip = Regex::new(r"(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?").unwrap();
    let re_hash_num = Regex::new(r"#\d+").unwrap();
    let re_ledger_close_time = Regex::new(r"(?:[^\d])\d{9}(?:[^\d]|$)").unwrap();
    let re_weight = Regex::new(r"weight -?\d{1,2}").unwrap();
    let re_percent = Regex::new(r"percent \d{1,3}").unwrap();
    let re_votes = Regex::new(r"\d{1,3} time votes").unwrap();
    let re_participants = Regex::new(r"\d{1,3} participants").unwrap();
    let re_ledger_id = Regex::new(r": \d+ <=").unwrap();
    let re_ledger_id_trail = Regex::new(r"<= \d+").unwrap();
    let re_advance_ledger_id = Regex::new(r"\d+ with >= \d+").unwrap();
    let re_ledger_json_log = Regex::new(r"\{.+close_time_human.+\}").unwrap();
    let re_proposers = Regex::new(r"Proposers:\d{1,3}").unwrap();
    let re_thresh_weight = Regex::new(r"nw:\d{1,3}").unwrap();
    let re_thresh_vote = Regex::new(r"thrV:\d{1,3}").unwrap();
    let re_thresh_consensus = Regex::new(r"thrC:\d{1,3}").unwrap();
    let re_offset_estimate = Regex::new(r"is estimated at -?\d \(\d{1,3}\)").unwrap();
    let re_num_nodes = Regex::new(r"\d+ nodes").unwrap();
    let re_brackets_num = Regex::new(r"\[\d+\]").unwrap();
    let re_seq_num = Regex::new(r"seq=\d+").unwrap();
    let re_ledger_timeouts = Regex::new(r"\d+ timeouts for ledger \d+").unwrap();
    let re_missing_node = Regex::new(r"Missing node in \d+").unwrap();

    let mut started = false;

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                match_counter += 1;

                if !match_line(mtch.get(2).unwrap()) {
                    continue;
                }

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
                    rounds += 1;
                    if rounds == STOP_ROUNDS && STOP_ROUNDS != -1 {
                        break;
                    }
                }

                if !started {
                    continue;
                }

                // replace base-16 hashes of length 64 (e.g.: 58B57FBEF009EB802DA44B7B35E362DA33648FCD2FE3C3DA235C54EFC8A082A8)
                let msg_sanitized = &re_base_16.replace_all(msg, "#some-base-16-hash");
                // replace alpha numerical ids of length 52 (e.g.: nHBe4vqSAzjpPRLKwSFzRFtmvzXaf5wPPmuVrQCAoJoS1zskgDA4)
                let msg_sanitized = &re_alpha_num_id.replace_all(msg_sanitized, "#some-id");
                // replace ip addresses
                let msg_sanitized = &re_ip.replace_all(msg_sanitized, "#some-ip");
                // replace numbers with '#' prefix (e.g.: #5334)
                let msg_sanitized = &re_hash_num.replace_all(msg_sanitized, "#some-num");
                // replace ledger close times
                let msg_sanitized =
                    &re_ledger_close_time.replace_all(msg_sanitized, "#some-ledger-close-time");
                let msg_sanitized = &re_weight.replace_all(msg_sanitized, "#some-weight");
                let msg_sanitized = &re_percent.replace_all(msg_sanitized, "#some-percent");
                let msg_sanitized = &re_votes.replace_all(msg_sanitized, "#some-votes time votes");
                let msg_sanitized =
                    &re_participants.replace_all(msg_sanitized, "#some-participants");
                let msg_sanitized =
                    &re_ledger_id.replace_all(msg_sanitized, ": #some-ledger-id <=");
                let msg_sanitized =
                    &re_ledger_id_trail.replace_all(msg_sanitized, "<= #some-ledger-id");
                let msg_sanitized = &re_advance_ledger_id
                    .replace_all(msg_sanitized, "#some-ledger-id >= #validations");
                let msg_sanitized =
                    &re_ledger_json_log.replace_all(msg_sanitized, "LEDGER_STATUS_JSON_LOG");
                let msg_sanitized =
                    &re_proposers.replace_all(msg_sanitized, "Proposers:#some-proposers");
                let msg_sanitized =
                    &re_thresh_weight.replace_all(msg_sanitized, "#some-needweight");
                let msg_sanitized = &re_thresh_vote.replace_all(msg_sanitized, "#some-thresh-vote");
                let msg_sanitized =
                    &re_thresh_consensus.replace_all(msg_sanitized, "#some-thresh-consensus");
                let msg_sanitized = &re_offset_estimate.replace_all(
                    msg_sanitized,
                    "is estimated at #some-offset (#some-closecount)",
                );
                let msg_sanitized = &re_num_nodes.replace_all(msg_sanitized, "#num nodes");
                let msg_sanitized = &re_brackets_num.replace_all(msg_sanitized, "");
                let msg_sanitized = &re_seq_num.replace_all(msg_sanitized, "seq=#");
                let msg_sanitized = &re_ledger_timeouts
                    .replace_all(msg_sanitized, "# timeouts for ledger #some-ledger-id");
                let msg_sanitized =
                    &re_missing_node.replace_all(msg_sanitized, "Missing node in #some-ledger-id");

                let msg_sanitized = msg_sanitized.to_string();

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

    let mut prev: &u64 = &u64::max_value();
    let mut pprev: &u64 = &u64::max_value();

    let parsed_filename = filename.to_owned() + ".parsed";
    let parsed_file = File::create(parsed_filename)?;
    let mut parsed_file = BufWriter::new(parsed_file);

    let labeled_filename = filename.to_owned() + ".labeled";
    let labeled_file = File::create(labeled_filename)?;
    let mut labeled_file = BufWriter::new(labeled_file);

    writeln!(parsed_file, "{} {}", all_log_sequence.len(), log_list.len())?;
    writeln!(
        labeled_file,
        "{} {}",
        all_log_sequence.len(),
        log_list.len()
    )?;
    for item in all_log_sequence.iter() {
        write!(parsed_file, "1 {}", item.len())?;
        write!(labeled_file, "1 {}", item.len())?;

        for log_id in item.iter() {
            // If the previous 2 printed items are identical, don't print the result
            if log_id == prev && log_id == pprev {
            } else {
                write!(parsed_file, " {}", log_id)?;
                write!(
                    labeled_file,
                    " {}",
                    map_log(log_list.get(*log_id as usize).unwrap())
                )?;
            }
            // Shift the two previous values
            pprev = prev;
            prev = log_id;
        }
        writeln!(parsed_file)?;
        writeln!(labeled_file)?;
    }

    let mapping_filename = filename.to_owned() + ".mapping";
    let mapping_file = File::create(mapping_filename)?;

    write_mapping(mapping_file, log_list)?;

    // println!("total number of matches: {}", match_counter);
    // println!("total number of non-matches: {}", no_match_counter);

    Ok(())
}

fn write_mapping(out_file: File, log_list: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut out_file = BufWriter::new(out_file);

    for (id, log) in log_list.iter().enumerate() {
        writeln!(out_file, "{} {}", id, log)?;
    }

    Ok(())
}

fn map_log(log: &String) -> &str {
    match log
        .as_str()
        .get(log.find(" ").unwrap() + 1 as usize..log.len())
        .unwrap()
    {
        "Entering consensus process, watching, synced=no" => "enterConsensusWatch",
        "Entering consensus process, validating, synced=no" => "enterConsensusValidating",
        "View of consensus changed during open status=open,  mode=wrongLedger" => {
            "viewChangeOpenToWrongLedger"
        }
        "View of consensus changed during open status=open,  mode=proposing" => {
            "viewChangeOpenToProposing"
        }

        "Consensus mode change before=observing, after=switchedLedger" => {
            "modeObservingToSwitchedLedger"
        }
        "Consensus mode change before=switchedLedger, after=proposing" => {
            "modeSwitchedLedgerToProposing"
        }
        "Consensus mode change before=proposing, after=observing" => "modeProposingToObserving",
        "Consensus mode change before=observing, after=wrongLedger" => "modeObservingToWrongLedger",
        "Consensus mode change before=observing, after=observing" => "modeObservingToObserving",
        "Consensus mode change before=wrongLedger, after=proposing" => "modeWrongledgerToProposing",
        "Consensus mode change before=proposing, after=proposing" => "modeProposingToProposing",
        "Consensus mode change before=wrongLedger, after=wrongLedger" => {
            "modeWrongledgerToWrongledger"
        }

        "Converge cutoff (#some-participants)" => "convergeCutoff",
        "CNF buildLCL #some-base-16-hash" => "buildLCL",
        "We closed at#some-ledger-close-time" => "ClosedAt",
        "Our close offset is estimated at #some-offset (#some-closecount)" => "closeOffset",
        "Need consensus ledger #some-base-16-hash" => "needConsensus",
        "Entering consensus with: #some-base-16-hash" => "enterConsensus",
        "Correct LCL is: #some-base-16-hash" => "correctLCL",
        "LEDGER_STATUS_JSON_LOG" => "jsonStatus",
        "#some-base-16-hash to #some-base-16-hash" => "hashTohash",
        "Entering consensus process, validating, synced=yes" => "enterConsensus",
        "CNF Val #some-base-16-hash" => "cnfSomething",
        "Proposers:#some-proposers #some-needweight #some-thresh-vote #some-thresh-consensus" => {
            "proposersWeightThresholdLog"
        }
        "No change (NO) : #some-weight, #some-percent" => "noChangeNo",
        "No change (YES) : #some-weight, #some-percent" => "noChangeYes",
        "Position change: CTime#some-ledger-close-time tx #some-base-16-hash" => "positionChange",
        "#some-votes time votes for#some-ledger-close-time" => "votesForClosetime",
        "By the time we got #some-base-16-hash no peers were proposing it" => "noPeersHashPropose",
        "Consensus built old ledger: #some-ledger-id <= #some-ledger-id" => "buildOldLedger",
        "Bowing out of consensus" => "consensusBowOut",
        "Have the consensus ledger #some-base-16-hash" => "haveConsensusLedger",
        "We have TX consensus but not CT consensus" => "haveTXNotCTConsensus",
        "Advancing accepted ledger to #some-ledger-id >= #validations validations" => {
            "advancingLedger"
        }
        "Consensus time for #some-num with LCL #some-base-16-hash" => "consensusTimeWithLCL",
        "Transaction is obsolete" => "transactionObsolete",
        " GetLedger: Route TX set failed" => "routeTXSetFailed",
        "Not relaying trusted proposal" => "notRelayProposal",
        " Got request for #num nodes at depth 3, return #num nodes" => "gotRequest3Nodes",
        " Got request for #num nodes at depth 2, return #num nodes" => "gotRequest2Nodes",
        " Got request for #num nodes at depth 1, return #num nodes" => "gotRequest1Nodes",
        " Got request for #num nodes at depth 0, return #num nodes" => "gotRequest0Nodes",
        " Duplicate manifest #some-num" => "duplicateManifest",
        " Untrusted manifest #some-num" => "untristedManifest",
        "Want: #some-base-16-hash" => "wantHash",
        "# timeouts for ledger #some-ledger-id" => "timeoutForLedgerID",
        "Unable to determine hash of ancestor seq=# from ledger hash=#some-base-16-hash seq=#" => {
            "unableHashLedgerAncestor"
        }
        " Ledger/TXset data with no nodes" => "ledger/TXNoNodes",

        "STATE->full" => "stateFull",
        "STATE->tracking" => "stateTracking",
        "STATE->syncing" => "stateSyncing",
        "STATE->connected" => "stateConnected",

        "Net LCL #some-base-16-hash" => "netLCL",
        "Our LCL: " => "ourLCL",

        "Built fetch pack with #num nodes" => "builtFetchPack",
        " Bad manifest #some-num: stale" => "badManifestStale",
        " Unable to route TX/ledger data reply" => "unableRouteTX/LedgerReply",
        "Initiating consensus engine" => "initiateConsensusEngine",
        "Node count (2) is sufficient." => "nodeCountSufficient",
        "We are not running on the consensus ledger" => "notOnConsensusLedger",
        "time jump" => "timeJump",

        " getNodeFat( NodeID(3,#some-base-16-hash)) throws exception: AS node" => "getNodeFat",
        " getNodeFat( NodeID(5,#some-base-16-hash)) throws exception: AS node" => "getNodeFat",
        "Missing node in #some-ledger-id" => "missingNodeInLedgerID",
        "Missing node in #some-base-16-hash" => "missingNodeInHash",
        "TimeKeeper: Close time offset now -1" => "closeTimeOffset",

        _ => {
            println!("{}", log);
            "unknownLog"
        }
    }
}

fn match_line(mtch: Match) -> bool {
    // Match on all log categories
    let res = match mtch.as_str() {
        "NetworkOPs" => true,
        "LedgerConsensus" => true,
        "LedgerMaster" => true,
        "Protocol" => true,
        "Peer" => false,
        "Application" => false,
        "LoadManager" => true,
        "LoadMonitor" => false,
        "PeerFinder" => false,
        "ManifestCache" => false,
        "Server" => false,
        "Validations" => true,
        "Resource" => false,
        "Ledger" => true,
        "JobQueue" => true,
        "NodeStore" => true,
        "TaggedCache" => true,
        "Amendments" => true,
        "OrderBookDB" => true,
        "ValidatorList" => true,
        "ValidatorSite" => false,
        "Flow" => false,
        "TimeKeeper" => true,
        "InboundLedger" => true,
        "TransactionAcquire" => true,
        "LedgerHistory" => true,
        "OpenLedger" => true,
        "PathRequest" => true,
        "TxQ" => true,
        "Resolver" => true,
        "Overlay" => true,
        "LedgerCleaner" => true,
        unknown => {
            eprintln!("encountered unknown event \"{}\"", unknown);
            "unknown log";
            false
        }
    };

    return res;
}
