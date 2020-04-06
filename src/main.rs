use indicatif::ProgressBar;
use regex::Match;
use regex::Regex;
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::process;
use std::result::Result;

static LOG_ENTERING_CONSENSUS: &str = "LedgerConsensus:NFO Entering consensus process";
// Stop after number rounds
static STOP_ROUNDS: i32 = 100;
// Process entire file
// static STOP_ROUNDS: i32 = -1;

fn main() {
    if let Err(error) = try_main() {
        eprintln!("{}", error);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let bar = ProgressBar::new(STOP_ROUNDS.try_into().unwrap());
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

    // Shorter regex separating on spaces in the log line, first match is the entire line, 1 is the message, 2 is the origin, 3 is the level
    let re = Regex::new(r".{11}\s.{18}\s((\w+):(\w+)\s.+)").unwrap();

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
    let re_some_tasks = Regex::new(r"\d+ tasks").unwrap();
    let re_some_jobs = Regex::new(r"\d+ jobs").unwrap();
    let re_some_items = Regex::new(r"\d+ items").unwrap();
    let re_some_of_some = Regex::new(r"\d+  of \d+ listed").unwrap();
    let re_some_of = Regex::new(r"\d+ of").unwrap();
    let re_of_some_for = Regex::new(r"of \d+ for").unwrap();
    let re_some_some_id = Regex::new(r"\d+:#some-id").unwrap();
    let re_some_trusted = Regex::new(r"\d+ trusted").unwrap();
    let re_some_added = Regex::new(r"\d+ added").unwrap();
    let re_some_removed = Regex::new(r"\d+ removed").unwrap();
    let re_some_good_num = Regex::new(r"good:\d+").unwrap();
    let re_some_dupe_num = Regex::new(r"dupe:\d+").unwrap();
    let re_some_src = Regex::new(r"src=\d+").unwrap();
    let re_some_from = Regex::new(r"from \d+").unwrap();
    let re_some_n = Regex::new(r"n=\d+").unwrap();
    let re_some_peer = Regex::new(r"Peer [0-9A-F]+ votes").unwrap();
    let re_some_peer_now = Regex::new(r"Peer [0-9A-F]+ now").unwrap();
    let re_some_peer_has = Regex::new(r"[0-9A-F]+ has").unwrap();
    let re_some_peer_votes = Regex::new(r"votes \w+ on").unwrap();
    let re_some_transactions = Regex::new(r"\d+ transactions?").unwrap();
    let re_some_changes = Regex::new(r"\d+ changes").unwrap();
    let re_some_and = Regex::new(r"\d+ and").unwrap();
    let re_some_begins = Regex::new(r"\d+ begins").unwrap();
    let re_some_completed = Regex::new(r"\d+ completed").unwrap();
    let re_some_accounts = Regex::new(r"\d+ accounts?").unwrap();
    let re_is_some_nl = Regex::new(r"is \d+$").unwrap();
    let re_to_some_nl = Regex::new(r"to \d+$").unwrap();
    let re_hash_colon_some = Regex::new(r"#some-base-16-hash:\d+").unwrap();
    let re_some_branch_support_object = Regex::new(r"\{.+branchSupport.+}").unwrap();
    let re_agree_disagree = Regex::new(r"agree=\d+, disagree=\d+$").unwrap();
    let re_some_consensus_dbg = Regex::new(r"\(working seq.+quorum: \d+\)").unwrap();
    let re_report_some_prop = Regex::new(r"Prop=.+fail=[a-z]{2,3}$").unwrap();
    let re_progress_some = Regex::new(r"progress\(\d+\)").unwrap();
    let re_timeout_some = Regex::new(r"Timeout\(\d+\) pc=\d+ acquiring").unwrap();
    let re_held_some = Regex::new(r"held: -*\d+$").unwrap();
    let re_balance_some = Regex::new(r"Balance: \d+(\.\d+)?/[A-Z]{3}$").unwrap();
    let re_offer_out =
        Regex::new(r"Offer out: \d+(\.\d+)?/[A-Z]{3}( \(issuer: r[A-Za-z0-9]{24,34}\))?$").unwrap();
    let re_offer_in_some_issuer =
        Regex::new(r"Offer in: \d+(\.\d+)?/[A-Z]{3}( \(issuer: r[A-Za-z0-9]{24,34}\))?$").unwrap();
    let re_crossing_as_some = Regex::new(r"Crossing as: r[A-Za-z0-9]{25,35}$").unwrap();
    let re_attempting_cross_one =
        Regex::new(r"Attempting cross: r[A-Za-z0-9]{24,34}/[A-Z]{3} -> [A-Z]{3}$").unwrap();
    let re_attempting_cross_two =
        Regex::new(r"Attempting cross: [A-Z]{3} -> r[A-Za-z0-9]{24,34}/[A-Z]{3}$").unwrap();
    let re_attempting_cross_double = Regex::new(
        r"Attempting cross: r[A-Za-z0-9]{24,34}/[A-Z]{3} -> r[A-Za-z0-9]{24,34}/[A-Z]{3}$",
    )
    .unwrap();
    let re_final_result = Regex::new(r"final result: [a-z]+$").unwrap();
    let re_order_some_value = Regex::new(r"order \d+$").unwrap();
    let re_has_some_some_required = Regex::new(r"has \d+, \d+ required$").unwrap();
    let re_seq_some = Regex::new(r"seq \d+:?").unwrap();
    let re_some_nays_object = Regex::new(r"\{.+nays.+}").unwrap();
    let re_some_differences = Regex::new(r"\d+ differences").unwrap();
    let re_success_some = Regex::new(r"success \d+").unwrap();
    let re_some_processed = Regex::new(r"\d+ processed").unwrap();
    let re_ledger_some = Regex::new(r"Ledger \d+").unwrap();
    let re_account_some = Regex::new(r"r[a-zA-Z0-9]{25,35}").unwrap();
    let re_done_complete = Regex::new(r"complete \d+").unwrap();
    let re_fetch_pack = Regex::new(r"pack for \d+").unwrap();
    let re_num_out_of = Regex::new(r"\d+ out of \d+").unwrap();
    let re_books_found = Regex::new(r"\d+ books found").unwrap();
    let re_timeouts_some = Regex::new(r"timeouts:\d+").unwrap();
    let re_status_other_than = Regex::new(r"Status other than -?\d+").unwrap();
    let re_thresh_some = Regex::new(r"Thresh:\d+").unwrap();
    let re_save_for = Regex::new(r"save for \d+").unwrap();
    let re_ledger_obj = Regex::new(r"\{.+acquired.+}").unwrap();
    let re_some_failed_and_some = Regex::new(r"\d+ failed and \d+").unwrap();
    let re_node_count_some = Regex::new(r"Node count \(\d+\)").unwrap();

    let mut started = false;

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                let msg = mtch.get(1).unwrap().as_str();

                if msg.starts_with(LOG_ENTERING_CONSENSUS) {
                    rounds += 1;
                    bar.inc(1);
                    if rounds > STOP_ROUNDS && STOP_ROUNDS != -1 {
                        break;
                    }
                    all_log_sequence.push(Vec::new());
                    started = true;
                }

                if !match_line(mtch.get(2).unwrap(), mtch.get(3).unwrap()) {
                    continue;
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
                let msg_sanitized =
                    &re_some_peer.replace_all(msg_sanitized, "Peer #some-peer-node votes");
                let msg_sanitized =
                    &re_some_peer_now.replace_all(msg_sanitized, "Peer #some-peer-node now");
                let msg_sanitized =
                    &re_some_peer_has.replace_all(msg_sanitized, "#some-peer-node has");
                let msg_sanitized =
                    &re_some_peer_votes.replace_all(msg_sanitized, "votes #some-vote on");
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
                let msg_sanitized = &re_some_tasks.replace_all(msg_sanitized, "#some-tasks tasks");
                let msg_sanitized = &re_some_jobs.replace_all(msg_sanitized, "#some-jobs jobs");
                let msg_sanitized = &re_some_items.replace_all(msg_sanitized, "#some-items items");
                let msg_sanitized =
                    &re_some_of_some.replace_all(msg_sanitized, "#some of #some listed");
                let msg_sanitized = &re_some_of.replace_all(msg_sanitized, "#some of");
                let msg_sanitized = &re_of_some_for.replace_all(msg_sanitized, "of #some for");
                let msg_sanitized = &re_some_some_id.replace_all(msg_sanitized, "#some:#some-id");
                let msg_sanitized = &re_some_trusted.replace_all(msg_sanitized, "#some trusted");
                let msg_sanitized = &re_some_added.replace_all(msg_sanitized, "#some added");
                let msg_sanitized = &re_some_removed.replace_all(msg_sanitized, "#some removed");
                let msg_sanitized =
                    &re_some_good_num.replace_all(msg_sanitized, "good:#some-good-num");
                let msg_sanitized =
                    &re_some_dupe_num.replace_all(msg_sanitized, "dupe:#some-dupe-num");
                let msg_sanitized = &re_some_src.replace_all(msg_sanitized, "src=#some-src-num");
                let msg_sanitized = &re_some_from.replace_all(msg_sanitized, "from #some_number");
                let msg_sanitized = &re_some_n.replace_all(msg_sanitized, "n=#some-num");
                let msg_sanitized =
                    &re_some_transactions.replace_all(msg_sanitized, "#some transactions");
                let msg_sanitized = &re_some_changes.replace_all(msg_sanitized, "#some changes");
                let msg_sanitized = &re_some_and.replace_all(msg_sanitized, "#some and");
                let msg_sanitized = &re_some_begins.replace_all(msg_sanitized, "#some begins");
                let msg_sanitized =
                    &re_some_completed.replace_all(msg_sanitized, "#some completed");
                let msg_sanitized = &re_some_accounts.replace_all(msg_sanitized, "#some accounts");
                let msg_sanitized = &re_is_some_nl.replace_all(msg_sanitized, "is #some");
                let msg_sanitized = &re_to_some_nl.replace_all(msg_sanitized, "to #some");
                let msg_sanitized =
                    &re_hash_colon_some.replace_all(msg_sanitized, "#some-base-16-hash:#some");
                let msg_sanitized = &re_some_branch_support_object
                    .replace_all(msg_sanitized, "#some-branch-support-object");
                let msg_sanitized =
                    &re_agree_disagree.replace_all(msg_sanitized, "agree=#some, disagree=#some");
                let msg_sanitized =
                    &re_some_consensus_dbg.replace_all(msg_sanitized, "(#truncated)");
                let msg_sanitized = &re_report_some_prop.replace_all(
                    msg_sanitized,
                    "Prop=#some val=#some corLCL=#some fail=#some",
                );
                let msg_sanitized = &re_progress_some.replace_all(msg_sanitized, "progress(#some)");
                let msg_sanitized = &re_timeout_some
                    .replace_all(msg_sanitized, "Timeout(#some) pc=#some acquiring");
                let msg_sanitized = &re_held_some.replace_all(msg_sanitized, "held: #some");
                let msg_sanitized =
                    &re_balance_some.replace_all(msg_sanitized, "Balance: #some-value/#currency");
                let msg_sanitized =
                    &re_offer_out.replace_all(msg_sanitized, "Offer out: #some-value/#currency");
                let msg_sanitized = &re_offer_in_some_issuer
                    .replace_all(msg_sanitized, "Offer in: #some-value/#currency");
                let msg_sanitized =
                    &re_crossing_as_some.replace_all(msg_sanitized, "Crossing as: #some-id");
                let msg_sanitized = &re_attempting_cross_one.replace_all(
                    msg_sanitized,
                    "Attempting cross: #some-account/#currency -> #currency",
                );
                let msg_sanitized = &re_attempting_cross_two.replace_all(
                    msg_sanitized,
                    "Attempting cross: #currency -> #some-account/#currency",
                );
                let msg_sanitized = &re_attempting_cross_double.replace_all(
                    msg_sanitized,
                    "Attempting cross: #some-account/#currency -> #some-account/#currency",
                );
                // let msg_sanitized =
                //     &re_final_result.replace_all(msg_sanitized, "final result: #some");
                let msg_sanitized =
                    &re_order_some_value.replace_all(msg_sanitized, "order #some-value");
                let msg_sanitized = &re_has_some_some_required
                    .replace_all(msg_sanitized, "has #some, #some required");
                let msg_sanitized = &re_seq_some.replace_all(msg_sanitized, "seq #some:");
                let msg_sanitized = &re_some_nays_object.replace_all(msg_sanitized, "{truncated}");
                let msg_sanitized =
                    &re_some_differences.replace_all(msg_sanitized, "#some differences");
                let msg_sanitized = &re_success_some.replace_all(msg_sanitized, "success #some");
                let msg_sanitized =
                    &re_some_processed.replace_all(msg_sanitized, "#some processed");
                let msg_sanitized = &re_account_some.replace_all(msg_sanitized, "#some-account");
                let msg_sanitized = &re_ledger_some.replace_all(msg_sanitized, "Ledger #some");
                let msg_sanitized =
                    &re_done_complete.replace_all(msg_sanitized, "complete #some-num");
                // replace ledger close times
                let msg_sanitized =
                    &re_ledger_close_time.replace_all(msg_sanitized, "#some-ledger-close-time");
                let msg_sanitized = &re_fetch_pack.replace_all(msg_sanitized, "pack for #some-obj");
                let msg_sanitized = &re_num_out_of.replace_all(msg_sanitized, "#some out of #some");
                let msg_sanitized = &re_books_found.replace_all(msg_sanitized, "#some books found");
                let msg_sanitized = &re_timeouts_some.replace_all(msg_sanitized, "timeouts:#some");
                let msg_sanitized =
                    &re_status_other_than.replace_all(msg_sanitized, "Status other than #some");
                let msg_sanitized = &re_thresh_some.replace_all(msg_sanitized, "Thresh:#some");
                let msg_sanitized = &re_save_for.replace_all(msg_sanitized, "pack for #some");
                let msg_sanitized = &re_ledger_obj.replace_all(msg_sanitized, "{truncated}");
                let msg_sanitized =
                    &re_some_failed_and_some.replace_all(msg_sanitized, "#some failed and #some");
                let msg_sanitized =
                    &re_node_count_some.replace_all(msg_sanitized, "Node count (#some)");

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
                // eprintln!("found no match in line: {}", l);
            }
        }
    }

    bar.finish();

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
        let mut log_string: String = "".to_owned();
        let mut labeled_log_string: String = "".to_owned();

        // Build the string to log
        for log_id in item.iter() {
            // If the previous 2 printed items are identical, don't print the result
            if log_id == prev && log_id == pprev {
            } else {
                // Add the id's to the line
                log_string.push_str(format!(" {}", log_id).as_str());
                // Add the labels to the line
                labeled_log_string.push_str(
                    format!(" {}", map_log(log_list.get(*log_id as usize).unwrap())).as_str(),
                );
            }
            // Shift the two previous values
            pprev = prev;
            prev = log_id;
        }

        // Split the string on spaces and count the amount of entries
        let len = log_string.trim().split(" ").collect::<Vec<&str>>().len();

        // Write to all files
        write!(parsed_file, "1 {}", len)?;
        write!(parsed_file, "{}", log_string)?;
        write!(labeled_file, "1 {}", len)?;
        write!(labeled_file, "{}", labeled_log_string)?;

        writeln!(parsed_file)?;
        writeln!(labeled_file)?;
    }

    let mapping_filename = filename.to_owned() + ".mapping";
    let mapping_file = File::create(mapping_filename)?;

    write_mapping(mapping_file, log_list)?;

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
        .trim()
    {
        "Entering consensus process, watching, synced=no" => "enterConsensusWatch",
        "Entering consensus process, validating, synced=no" => "enterConsensusValidating",
        "View of consensus changed during open status=open,  mode=wrongLedger" => {
            "viewChangeOpenToWrongLedger"
        }
        "View of consensus changed during open status=open,  mode=proposing" => {
            "viewChangeOpenToProposing"
        }
        "View of consensus changed during establish status=establish,  mode=proposing" => "viewChangeEstablishProposing",

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
        "GetLedger: Route TX set failed" => "routeTXSetFailed",
        "Not relaying trusted proposal" => "notRelayProposal",
        "Got request for #num nodes at depth 3, return #num nodes" => "gotRequest3Nodes",
        "Got request for #num nodes at depth 2, return #num nodes" => "gotRequest2Nodes",
        "Got request for #num nodes at depth 1, return #num nodes" => "gotRequest1Nodes",
        "Got request for #num nodes at depth 0, return #num nodes" => "gotRequest0Nodes",
        "Duplicate manifest #some-num" => "duplicateManifest",
        "Untrusted manifest #some-num" => "untristedManifest",
        "Want: #some-base-16-hash" => "wantHash",
        "# timeouts for ledger #some-ledger-id" => "timeoutForLedgerID",
        "Unable to determine hash of ancestor seq=# from ledger hash=#some-base-16-hash seq=#" => {
            "unableHashLedgerAncestor"
        }
        "Ledger/TXset data with no nodes" => "ledgerOrTXNoNodes",

        "STATE->full" => "stateFull",
        "STATE->tracking" => "stateTracking",
        "STATE->syncing" => "stateSyncing",
        "STATE->connected" => "stateConnected",

        "Net LCL #some-base-16-hash" => "netLCL",
        "Our LCL:" => "ourLCL",
        "LCL is #some-base-16-hash" => "lclIs",

        "Built fetch pack with #num nodes" => "builtFetchPack",
        "Bad manifest #some-num: stale" => "badManifestStale",
        "Unable to route TX/ledger data reply" => "unableRouteTXOrLedgerReply",
        "Initiating consensus engine" => "initiateConsensusEngine",
        "Node count (2) is sufficient." => "nodeCountSufficient",
        "We are not running on the consensus ledger" => "notOnConsensusLedger",
        "time jump" => "timeJump",

        "getNodeFat( NodeID(3,#some-base-16-hash)) throws exception: AS node" => "getNodeFat",
        "getNodeFat( NodeID(5,#some-base-16-hash)) throws exception: AS node" => "getNodeFat",
        "Missing node in #some-ledger-id" => "missingNodeInLedgerID",
        "Missing node in #some-base-16-hash" => "missingNodeInHash",
        "TimeKeeper: Close time offset now -1" => "closeTimeOffset",
        "Not relaying UNTRUSTED proposal" => "notReplayingUntrustedProposal",
        "Ignoring incoming transaction: Need network ledger" => "ignoringIncomingNeedNetwork",
        "Got proposal for #some-base-16-hash but we are on #some-base-16-hash" => {
            "gotProposalButAreOn"
        }
        "normal consensus" => "normalConsensus",
        "Ledger not found: WHERE LedgerHash = '#some-base-16-hash'" => "ledgerNotFound",
        "Need validated ledger for preferred ledger analysis #some-base-16-hash" => {
            "needValidatedLedger"
        }
        "No validated ledger" => "noValidatedLedger",
        "Deferring InboundLedger timer due to load" => "deferringLedgerDueToLoad",
        "GetLedger: Routing Tx set request" => "getLedgerRoutingTxSet",
        "Starting" => "starting",
        "Started" => "started",
        "Initializing" => "initializing",

        "Ledger AS node stats: good:#some-good-num" => "ledgerAsNodeStatsGood",
        "Ledger AS node stats: dupe:#some-dupe-num" => "ledgerAsNodeStatsDupe",
        "Ledger AS node stats: good:#some-good-num dupe:#some-dupe-num" => "ledgerAsNodeStatsGoodDupe",
        "Val for #some-base-16-hash trusted/full from #some-id signing key #some-id current src=#some-src-num" => "valHashTrustedFullCurrent",
        "recvValidation #some-base-16-hash from #some_number" => "recvValidation",
        "Val for #some-base-16-hash from #some-id not added UNlisted" => "valHashNotAddedUNlisted",
        "GetLedger: Can't provide map" => "getLedgerCantProvideMap",
        "#some of #some listed validators eligible for inclusion in the trusted set" => "numValidatorsInclusionTrustset",
        "Consensus built new ledger" => "consensusBuiltNewLedger",
        "Built ledger #some-num: #some-base-16-hash" => "buildLedger",
        "Building canonical tx set: #some-base-16-hash" => "buildingCanonicalTxSet",
        "Report: Transaction Set = #some-base-16-hash, close#some-ledger-close-time" => "reportTransactionSetClose",
        "GetLedger: Request routed" => "getLedgerRequestRouted",
        "L: #some-base-16-hash n=#some-num" => "lHashNval",
        "GetLedger: Large send queue" => "getLedgerLargeSendQueue",
        "GetObject: Large send queue" => "getObjectLargeSendQueue",
        "Transaction is now included in open ledger" => "transactionIncluded",
        "Peer #some-peer-node votes votes NO on #some-base-16-hash" => "peerVotesNo",
        "Ledger TX node stats: good:#some-good-num" => "ledgerTxNodeStatsGood",
        "Got tx #some-base-16-hash" => "gotTxHash",
        "Peer #some-peer-node now votes #some-vote on #some-base-16-hash" => "somePeerVote",
        "Peer #some-peer-node votes #some-vote on #some-base-16-hash" => "peerVotesOn",
        "#some-peer-node has #some-base-16-hash" => "peerHasHash",
        "Tx: #some-base-16-hash" => "txHash",
        "TXN #some-base-16-hash/retry" => "txnRetry",
        "TXN #some-base-16-hash/final" => "txnFinal",
        "Entering RippleCalc in payment: #some-base-16-hash" => "enteringRippleCalc",
        "Transaction retry: Path could not send partial amount." => "retryCouldNotSendPartial",
        "Transaction applied: Path could not send partial amount." => "appliedCouldNotSendPartial",
        "Transaction applied: The transaction was applied. Only final in a validated ledger." => "appliedOnlyInFinal",
        "Not relaying disputed tx #some-base-16-hash" => "noReplayDisputedTx",
        "Don't have tx set for peer" => "noTxSetForPeer",
        "Test applying disputed transaction that did not get in #some-base-16-hash" => "testApplyDisputed",
        "createDisputes #some-base-16-hash to #some-base-16-hash" => "createDisputes",
        "Consensus built ledger we already had" => "consensusBuiltLedgerWeHad",
        "Transaction #some-base-16-hash is disputed" => "transactionIsDisputed",
        "Acquired TX set #some-base-16-hash" => "acquiredTxSetHash",
        "Consensus built ledger we were acquiring" => "consensusBuiltLedgerWeAcquired",
        "Taker Crossing as: #some-id" => "takerCrossingAsId",
        "Taker    Offer in: #some-value/#currency" => "takerOfferIn",
        "Taker   Offer out: #some-value/#currency" => "takerOfferOut",
        "Taker     Balance: #some-value/#currency" => "takerOfferBalance",
        "Create cancels order #some-value" => "createCancelsOrder",
        "Attempting cross: #some-account/#currency -> #currency" => "attemptCrossCurrency",
        "Attempting cross: #currency -> #some-account/#currency" => "attemptCrossCurrency",
        "Attempting cross: #some-account/#currency -> #some-account/#currency" => "attemptCrossCurrency",
        "final result: success" => "finalResultSuccess",
        "{truncated}" => "ledgerInfoLog",
        "#some differences found" => "someDifferences",
        "CCTime: seq #some: #some-peer-node has #some, #some required" => "cctimeSeqRequired",
        "Taker    Offer in:#some-ledger-close-timeXRP" => "takerOfferInLedgerClose",
        "Taker   Offer out:#some-ledger-close-timeXRP" => "takerOfferOutLedgerClose",
        "Status other than success #some" => "statusOtherSuccess",
        "We now vote YES on #some-base-16-hash" => "nowVoteYes",
        "We now vote NO on #some-base-16-hash" => "nowVoteNo",
        "Timeout(#some) pc=#some acquiring #some-base-16-hash" => "timeoutPcAcquiring",
        "Pass: #some begins (#some transactions)" => "passSomeBegins",
        "Pass: #some completed (#some changes)" => "passSomeCompleted",
        "Not creating disputes: no position yet." => "notCreatingDisputesNoPos",
        "Applied #some transactions." => "appliedTransactions",
        "Flushed #some accounts and #some transactions nodes" => "flushedAccountsAndNodes",
        "Ledger #some-peer-node has #some transactions. Ledgers are processing as expected. Expected transactions is currently #some and multiplier is #some" => "expectedTransactionsMul",
        "Final pass: #some begins (#some transactions)" => "finalPassBegins",
        "Final pass: #some completed (#some changes)" => "finalPassCompleted",
        "Expected transactions updated to #some and multiplier updated to #some" => "exectedTransactions",
        "Transaction should be held: #some" => "transactionShouldHeld",
        "ValidationTrie #some-branch-support-object" => "validationTrieBranch",
        "Queued transaction #some-base-16-hash rules or flags have changed. Flags from #some_number to #some" => "queuedTxRulesChanged",
        "Queued transaction #some-base-16-hash applied successfully with tecPATH_DRY. Remove from queue." => "queuedTxAppliedPathDry",
        "Queued transaction #some-base-16-hash applied successfully with tesSUCCESS. Remove from queue." => "queuedTxAppliedSuccess",
        "Transaction is likely to claim a fee, but is queued until fee drops" => "txFeeQueued",
        "Trying to cancel offer #some-num" => "tryCancelOffer",
        "Proposal: Dropping UNTRUSTED (load)" => "proposalDropUntrusted",
        "Validation: Dropping UNTRUSTED (load)" => "validationDropUntrusted",
        "Added transaction #some-base-16-hash with result tesSUCCESS from existing account #some-account to queue. Flags: 0" => "addedTxSuccessAccount",
        "Added transaction #some-base-16-hash with result tesSUCCESS from new account #some-account to queue. Flags: 0" => "addedTxSuccessNewAccount",
        "Attempting to apply #some transactions" => "attemptApplyTxs",
        "not pausing (#truncated)" => "notPausing",
        "Checking for TX consensus: agree=#some, disagree=#some" => "checkingTxConsensus",
        "Report: Prop=#some val=#some corLCL=#some fail=#some" => "reportPropvalColLCLFail",
        "Report: Prev = #some-base-16-hash:#some" => "reportPrev",
        "Acquire #some-base-16-hash timeouts:1 good:#some-good-num dupe:#some-dupe-num" => "acquireHashTimeoutGoodDupe",
        "Using quorum of #some for new set of #some trusted validators (#some added, #some removed)" => "UseQuorumNewValidators",
        "MATCH: seq=#" => "matchSeq",
        "tryAdvance publishing seq #some:" => "tryAdvancePublish",
        "Ledger #some accepted :#some-base-16-hash" => "ledgerAcceptedHash",
        "updateAll complete: #some processed and #some removed" => "upgradeAllComplete",
        "No progress(#some) for ledger #some-base-16-hash" => "noProgressLedger",
        "Done: complete #some-num" => "doneComplete",
        "Val for #some-base-16-hash trusted/full from #some-id signing key #some-id current src=local" => "valTrustedFullCurrent",
        "Consensus ledger fully validated" => "consensusLedgerFullyValidated",
        "Can't get seq #some: from #some_number past" => "cantGetSeqFrom",
        "Relaying disputed tx #some-base-16-hash" => "replayingDisputedTx",
        "Ledger TX node stats: dupe:#some-dupe-num" => "ledgerTxNodeStatsDupe",
        "Acquire #some-base-16-hash good:#some-good-num dupe:#some-dupe-num" => "acquireHashGoodDupe",
        "activated #some-ip (#some:#some-id)" => "activatedIp",
        "Had everything locally" => "everythingLocal",
        "Acquire #some-base-16-hash timeouts:1 no nodes processed" => "acquireTimeoutNoNodes",
        "Trigger on ledger: #some-base-16-hash completed" => "triggerLedgerHashCompleted",
        "Acquire #some-base-16-hash timeouts:1 good:#some-good-num" => "acquireTimeoutGood",
        "Offer #some-num can't be found." => "offerNotFound",
        "Queued transaction #some-base-16-hash failed with tefPAST_SEQ. Remove from queue." => "queuedTxFailedPastSeq",
        "TMManifest, #some-items items" => "manifestItems",
        "Val for #some-base-16-hash UNtrusted/full from #some-id signing key #some-id current src=#some-src-num" => "valUntrustedFullSigning",
        "Node on our acquiring TX set is TXN we may not have" => "nodeAcquiringTxMayNotHave",
        "Transaction retry: The source account does not exist." => "txRetrySourceNonExist",
        "Got root TXS node, already have it" => "gotRootTxsHaveIt",
        "Acquire #some-base-16-hash abort timeouts:1 good:#some-good-num dupe:#some-dupe-num" => "acquireAbortTimeout",
        "Acquire #some-base-16-hash abort timeouts:4 good:#some-good-num dupe:#some-dupe-num" => "acquireAbortTimeout",
        "activated [::ffff:#some-ip]:51235 (#some:#some-id)" => "activatedIp",
        "Consensus triggered check of ledger" => "consensusTriggeredLedgerCheck",
        "Acquire #some-base-16-hash timeouts:3 good:#some-good-num dupe:#some-dupe-num" => "acquireTimeout",
        "Acquire #some-base-16-hash timeouts:2 good:#some-good-num dupe:#some-dupe-num" => "acquireTimeout",
        "Acquire #some-base-16-hash abort timeouts:3 good:#some-good-num dupe:#some-dupe-num" => "acquireAbortTimeout",
        "GetObj: Late fetch pack for #some-obj" => "getObjLateFetch",
        "GetObj: Partial fetch pack for #some-obj" => "getObjPartialFetch",
        "Acquire #some-base-16-hash no nodes processed" => "acquireNoNodes",
        "Ledger #some-peer-node has #some transactions. Ledgers are processing slowly. Expected transactions is currently #some and multiplier is #some" => "ledgerHashTxsProcessingSlow",
        "Status: Out of sync" => "statusOutOfSync",
        "Advancing from #some_number to #some" => "advanceFromTo",
        "OrderBookDB::update>" => "orderBookUpdate",
        "#some-branch-support-object" => "someBranchSupportobject",
        "Val for #some-base-16-hash trusted/partial from #some-id signing key #some-id current src=local" => "valTrustedPartialCurrent",
        "GetObj: Full fetch pack for #some-obj" => "getObjFullFetch",
        "Swept #some out of #some inbound ledgers." => "sweptSomeLedgers",
        "Must wait minimum time before closing" => "mustWaitMinBeforeClosing",
        "OrderBookDB::update< #some books found" => "someBooksFound",

        _ => {
            println!("no mapping for log: {}", log);
            "unknownLog"
        }
    }
}

fn match_line(origin: Match, level: Match) -> bool {
    let is_debug = match level.as_str() {
        "DBG" => true,
        _unknown => false,
    };
    if !is_debug {
        return false;
    }

    // Match on all log categories
    let res = match origin.as_str() {
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
        "OpenLedger" => false,
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
