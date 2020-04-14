#![recursion_limit = "16384"]
#[macro_use]
extern crate lazy_static;

use indicatif::ProgressBar;
use regex::Captures;
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
static ROUNDS_PER_BATCH: i32 = 50;
static AMOUNT_BATCHES: i32 = 10;
// Process entire file
// static STOP_ROUNDS: i32 = -1;

fn main() {
    if let Err(error) = try_main() {
        eprintln!("{}", error);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let bar = ProgressBar::new((ROUNDS_PER_BATCH * AMOUNT_BATCHES).try_into().unwrap());
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

    let mut rounds = 0;

    // Count distinct logs
    let mut log_id_counter = 0;
    // Map log_string -> log_id
    let mut log_id_map: HashMap<String, u64> = HashMap::new();
    let mut all_log_sequence = Vec::<Vec<u64>>::new();
    let mut log_list = Vec::<String>::new();
    let mut log_counts = Vec::<u64>::new();

    // Regex separating on spaces in the log line, first match is the entire line, 1 is the message, 2 is the origin, 3 is the level
    let re = Regex::new(r".{11}\s.{18}\s((\w+):(\w+)\s.+)").unwrap();

    let mut started = false;

    while let Some(line) = contents.next() {
        let l = line.expect("end of file");
        let capture_res = re.captures(l.as_str());
        match capture_res {
            Some(mtch) => {
                let msg = mtch.get(1).unwrap().as_str();

                if msg.starts_with(LOG_ENTERING_CONSENSUS) {
                    if rounds > 0 && rounds % ROUNDS_PER_BATCH == 0 && ROUNDS_PER_BATCH != -1 {
                        let round_filename = format!(
                            "{}_rounds_{:03}_{:03}",
                            filename,
                            rounds - ROUNDS_PER_BATCH,
                            rounds - 1
                        );
                        all_log_sequence = clean_all_log_sequence(all_log_sequence);
                        write_files(&round_filename, &all_log_sequence, &log_list)?;
                        all_log_sequence = Vec::<Vec<u64>>::new();
                        all_log_sequence.push(Vec::new());

                        if rounds == AMOUNT_BATCHES * ROUNDS_PER_BATCH {
                            break;
                        }
                    }
                    all_log_sequence.push(Vec::new());
                    started = true;
                    rounds += 1;
                    bar.inc(1);
                }

                if !match_line(mtch) {
                    continue;
                }

                if !started {
                    continue;
                }

                let msg_sanitized = sanitize_message(msg);

                // if this is a new log
                let mut is_new = false;
                if !log_id_map.contains_key(&msg_sanitized) {
                    is_new = true;
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
                let log_id = log_id_map.get(&msg_sanitized).unwrap();

                // Skip log if no mapping is defined
                if map_log(log_id, &log_list, is_new).is_empty() {
                    continue;
                }

                // increase the count
                *(log_counts.get_mut(*log_id as usize).unwrap()) += 1;

                // append the id to the current sequence, if none found, add a new one
                let log_index = all_log_sequence.len() - 1;
                all_log_sequence.get_mut(log_index).unwrap().push(*log_id);
            }
            None => {
                // eprintln!("found no match in line: {}", l);
            }
        }
    }

    bar.finish();

    // dbg!(all_log_sequence);
    // dbg!(log_list);

    Ok(())
}

fn write_files(
    filename: &String,
    all_log_sequence: &Vec<Vec<u64>>,
    log_list: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_filename = filename.to_owned() + ".parsed";
    let parsed_file = File::create(parsed_filename)?;
    let mut parsed_file = BufWriter::new(parsed_file);

    let labeled_filename = filename.to_owned() + ".labeled";
    let labeled_file = File::create(labeled_filename)?;
    let mut labeled_file = BufWriter::new(labeled_file);

    let length = all_log_sequence.len();
    let alphabet_size = log_list.len();
    writeln!(parsed_file, "{} {}", length, alphabet_size)?;
    writeln!(labeled_file, "{} {}", length, alphabet_size)?;
    for item in all_log_sequence.iter() {
        // Write to all files
        let len = item.len();
        write!(parsed_file, "1 {}", len)?;
        write!(labeled_file, "1 {}", len)?;

        // Write log ids and labels to file
        for log_id in item.iter() {
            // Add the id's to the line
            write!(parsed_file, " {}", log_id)?;
            // Add the labels to the line
            let log_label = map_log(log_id, log_list, false);
            write!(labeled_file, " {}", log_label)?;
        }

        writeln!(parsed_file)?;
        writeln!(labeled_file)?;
    }

    let mapping_filename = filename.to_owned() + ".mapping";
    let mapping_file = File::create(mapping_filename)?;

    return write_mapping(mapping_file, log_list);
}

fn clean_all_log_sequence(all_log_sequence: Vec<Vec<u64>>) -> Vec<Vec<u64>> {
    let mut new_all_log_sequence = Vec::<Vec<u64>>::new();

    for item in all_log_sequence.iter() {
        let mut new_sequence = Vec::new();

        let mut prev: &u64 = &u64::max_value();
        let mut pprev: &u64 = &u64::max_value();

        // Build the string to log
        for log_id in item.iter() {
            // If the previous 2 log ids are identical, don't add it again
            if log_id == prev && log_id == pprev {
            } else {
                new_sequence.push(*log_id);
            }
            // Shift the two previous values
            pprev = prev;
            prev = log_id;
        }

        new_all_log_sequence.push(new_sequence);
    }

    return new_all_log_sequence;
}

fn write_mapping(out_file: File, log_list: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut out_file = BufWriter::new(out_file);

    for (id, log) in log_list.iter().enumerate() {
        writeln!(out_file, "{} {}", id, log)?;
    }

    Ok(())
}

fn map_log(log_id: &u64, log_list: &std::vec::Vec<std::string::String>, is_new: bool) -> String {
    let log = log_list.get(*log_id as usize).unwrap();
    let res = match log
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
        "Acquire #some-base-16-hash abort timeouts:#some good:#some-good-num dupe:#some-dupe-num" => "acquireAbortTimeout",
        "Acquire #some-base-16-hash timeouts:#some good:#some-good-num dupe:#some-dupe-num" => "acquireTimeoutGoodDupe",
        "Acquire #some-base-16-hash timeouts:#some good:#some-good-num" => "acquireTimeoutGood",
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
        "Transaction retry: Missing/inapplicable prior transaction." => "txRetryMissingPriorTx",
        "Transaction retry: Insufficient XRP balance to send." => "txRetryInsufficientBalance",
        "Transaction retry: A destination tag is required." => "txRetryDestTagRequired",
        // "Taker     Balance: #amount/#currency" => "olTakerBalance",
        // "Taker    Offer in: #amount/#currency" => "olTakerOfferIn",
        // "Taker   Offer out: #amount/#currency (issuer: #some-account)" => "olTakerOfferOut",
        // // "Taker     Balance: #amount/#currency" => "lcTakerBalance",
        // "Taker    Offer in: #amount/#currency (issuer: #some-account)" => "lcTakerOfferIn",
        // "Taker   Offer out: #amount/#currency" => "lcTakerOfferOut",

        _ => {
            println!("no mapping for log: {}", log);
            return String::from("");
        }
    };
    return String::from(res);
}

fn match_line(mtch: Captures) -> bool {
    let origin = mtch.get(2).unwrap();
    let level = mtch.get(3).unwrap();
    let res = match (origin.as_str(), level.as_str()) {
        ("NetworkOPs", _) => true,
        ("LedgerConsensus", _) => true,
        ("LedgerMaster", _) => true,
        ("Protocol", _) => true,
        ("Peer", _) => false,
        ("Application", _) => false,
        ("LoadManager", _) => false,
        ("LoadMonitor", _) => false,
        ("PeerFinder", _) => false,
        ("ManifestCache", _) => false,
        ("Server", _) => false,
        ("Validations", _) => true,
        ("Resource", _) => false,
        ("Ledger", _) => true,
        ("JobQueue", _) => true,
        ("NodeStore", _) => true,
        ("TaggedCache", _) => true,
        ("Amendments", _) => true,
        ("OrderBookDB", _) => true,
        ("ValidatorList", _) => true,
        ("ValidatorSite", _) => false,
        ("Flow", _) => false,
        ("TimeKeeper", _) => true,
        ("InboundLedger", _) => true,
        ("TransactionAcquire", _) => true,
        ("LedgerHistory", _) => true,
        ("OpenLedger", _) => false,
        ("PathRequest", _) => true,
        ("TxQ", _) => true,
        ("Resolver", _) => false,
        ("Overlay", _) => false,
        ("LedgerCleaner", _) => false,
        (unknown, _) => {
            eprintln!("encountered unknown event \"{}\"", unknown);
            "unknown log";
            false
        }
    };

    return res;
}

fn sanitize_message(msg: &str) -> String {
    lazy_static! {
        static ref RE_BASE_16: Regex = Regex::new(r"[0-9A-F]{64}").unwrap();
        static ref RE_ALPHA_NUM_ID: Regex = Regex::new(r"[A-Za-z0-9]{52}").unwrap();
        static ref RE_IP: Regex = Regex::new(r"(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?").unwrap();
        static ref RE_HASH_NUM: Regex = Regex::new(r"#\d+").unwrap();
        static ref RE_LEDGER_CLOSE_TIME: Regex = Regex::new(r"(?:[^\d])\d{9}(?:[^\d]|$)").unwrap();
        static ref RE_WEIGHT: Regex = Regex::new(r"weight -?\d{1,2}").unwrap();
        static ref RE_PERCENT: Regex = Regex::new(r"percent \d{1,3}").unwrap();
        static ref RE_VOTES: Regex = Regex::new(r"\d{1,3} time votes").unwrap();
        static ref RE_PARTICIPANTS: Regex = Regex::new(r"\d{1,3} participants").unwrap();
        static ref RE_LEDGER_ID: Regex = Regex::new(r": \d+ <=").unwrap();
        static ref RE_LEDGER_ID_TRAIL: Regex = Regex::new(r"<= \d+").unwrap();
        static ref RE_ADVANCE_LEDGER_ID: Regex = Regex::new(r"\d+ with >= \d+").unwrap();
        static ref RE_LEDGER_JSON_LOG: Regex = Regex::new(r"\{.+close_time_human.+\}").unwrap();
        static ref RE_PROPOSERS: Regex = Regex::new(r"Proposers:\d{1,3}").unwrap();
        static ref RE_THRESH_WEIGHT: Regex = Regex::new(r"nw:\d{1,3}").unwrap();
        static ref RE_THRESH_VOTE: Regex = Regex::new(r"thrV:\d{1,3}").unwrap();
        static ref RE_THRESH_CONSENSUS: Regex = Regex::new(r"thrC:\d{1,3}").unwrap();
        static ref RE_OFFSET_ESTIMATE: Regex =
            Regex::new(r"is estimated at -?\d \(\d{1,3}\)").unwrap();
        static ref RE_NUM_NODES: Regex = Regex::new(r"\d+ nodes").unwrap();
        static ref RE_BRACKETS_NUM: Regex = Regex::new(r"\[\d+\]").unwrap();
        static ref RE_SEQ_NUM: Regex = Regex::new(r"seq=\d+").unwrap();
        static ref RE_LEDGER_TIMEOUTS: Regex = Regex::new(r"\d+ timeouts for ledger \d+").unwrap();
        static ref RE_MISSING_NODE: Regex = Regex::new(r"Missing node in \d+").unwrap();
        static ref RE_SOME_TASKS: Regex = Regex::new(r"\d+ tasks").unwrap();
        static ref RE_SOME_JOBS: Regex = Regex::new(r"\d+ jobs").unwrap();
        static ref RE_SOME_ITEMS: Regex = Regex::new(r"\d+ items").unwrap();
        static ref RE_SOME_OF_SOME: Regex = Regex::new(r"\d+  of \d+ listed").unwrap();
        static ref RE_SOME_OF: Regex = Regex::new(r"\d+ of").unwrap();
        static ref RE_OF_SOME_FOR: Regex = Regex::new(r"of \d+ for").unwrap();
        static ref RE_SOME_SOME_ID: Regex = Regex::new(r"\d+:#some-id").unwrap();
        static ref RE_SOME_TRUSTED: Regex = Regex::new(r"\d+ trusted").unwrap();
        static ref RE_SOME_ADDED: Regex = Regex::new(r"\d+ added").unwrap();
        static ref RE_SOME_REMOVED: Regex = Regex::new(r"\d+ removed").unwrap();
        static ref RE_SOME_GOOD_NUM: Regex = Regex::new(r"good:\d+").unwrap();
        static ref RE_SOME_DUPE_NUM: Regex = Regex::new(r"dupe:\d+").unwrap();
        static ref RE_SOME_SRC: Regex = Regex::new(r"src=\d+").unwrap();
        static ref RE_SOME_FROM: Regex = Regex::new(r"from \d+").unwrap();
        static ref RE_SOME_N: Regex = Regex::new(r"n=\d+").unwrap();
        static ref RE_SOME_PEER: Regex = Regex::new(r"Peer [0-9A-F]+ votes").unwrap();
        static ref RE_SOME_PEER_NOW: Regex = Regex::new(r"Peer [0-9A-F]+ now").unwrap();
        static ref RE_SOME_PEER_HAS: Regex = Regex::new(r"[0-9A-F]+ has").unwrap();
        static ref RE_SOME_PEER_VOTES: Regex = Regex::new(r"votes \w+ on").unwrap();
        static ref RE_SOME_TRANSACTIONS: Regex = Regex::new(r"\d+ transactions?").unwrap();
        static ref RE_SOME_CHANGES: Regex = Regex::new(r"\d+ changes").unwrap();
        static ref RE_SOME_AND: Regex = Regex::new(r"\d+ and").unwrap();
        static ref RE_SOME_BEGINS: Regex = Regex::new(r"\d+ begins").unwrap();
        static ref RE_SOME_COMPLETED: Regex = Regex::new(r"\d+ completed").unwrap();
        static ref RE_SOME_ACCOUNTS: Regex = Regex::new(r"\d+ accounts?").unwrap();
        static ref RE_IS_SOME_NL: Regex = Regex::new(r"is \d+$").unwrap();
        static ref RE_TO_SOME_NL: Regex = Regex::new(r"to \d+$").unwrap();
        static ref RE_HASH_COLON_SOME: Regex = Regex::new(r"#some-base-16-hash:\d+").unwrap();
        static ref RE_SOME_BRANCH_SUPPORT_OBJECT: Regex =
            Regex::new(r"\{.+branchSupport.+}").unwrap();
        static ref RE_AGREE_DISAGREE: Regex = Regex::new(r"agree=\d+, disagree=\d+$").unwrap();
        static ref RE_SOME_CONSENSUS_DBG: Regex =
            Regex::new(r"\(working seq.+quorum: \d+\)").unwrap();
        static ref RE_REPORT_SOME_PROP: Regex = Regex::new(r"Prop=.+fail=[a-z]{2,3}$").unwrap();
        static ref RE_PROGRESS_SOME: Regex = Regex::new(r"progress\(\d+\)").unwrap();
        static ref RE_TIMEOUT_SOME: Regex = Regex::new(r"Timeout\(\d+\) pc=\d+ acquiring").unwrap();
        static ref RE_HELD_SOME: Regex = Regex::new(r"held: -*\d+$").unwrap();
        static ref RE_BALANCE_SOME: Regex = Regex::new(r"Balance: \d+(\.\d+)?/[A-Z]{3}$").unwrap();
        static ref RE_OFFER_OUT: Regex =
            Regex::new(r"Offer out: \d+(\.\d+)?/[A-Z]{3}( \(issuer: r[A-Za-z0-9]{24,34}\))?$")
                .unwrap();
        static ref RE_OFFER_IN_SOME_ISSUER: Regex =
            Regex::new(r"Offer in: \d+(\.\d+)?/[A-Z]{3}( \(issuer: r[A-Za-z0-9]{24,34}\))?$")
                .unwrap();
        static ref RE_CROSSING_AS_SOME: Regex =
            Regex::new(r"Crossing as: r[A-Za-z0-9]{25,35}$").unwrap();
        static ref RE_ATTEMPTING_CROSS_ONE: Regex =
            Regex::new(r"Attempting cross: r[A-Za-z0-9]{24,34}/[A-Z]{3} -> [A-Z]{3}$").unwrap();
        static ref RE_ATTEMPTING_CROSS_TWO: Regex =
            Regex::new(r"Attempting cross: [A-Z]{3} -> r[A-Za-z0-9]{24,34}/[A-Z]{3}$").unwrap();
        static ref RE_ATTEMPTING_CROSS_DOUBLE: Regex = Regex::new(
            r"Attempting cross: r[A-Za-z0-9]{24,34}/[A-Z]{3} -> r[A-Za-z0-9]{24,34}/[A-Z]{3}$",
        )
        .unwrap();
        static ref RE_FINAL_RESULT: Regex = Regex::new(r"final result: [a-z]+$").unwrap();
        static ref RE_ORDER_SOME_VALUE: Regex = Regex::new(r"order \d+$").unwrap();
        static ref RE_HAS_SOME_SOME_REQUIRED: Regex =
            Regex::new(r"has \d+, \d+ required$").unwrap();
        static ref RE_SEQ_SOME: Regex = Regex::new(r"seq \d+:?").unwrap();
        static ref RE_SOME_NAYS_OBJECT: Regex = Regex::new(r"\{.+nays.+}").unwrap();
        static ref RE_SOME_DIFFERENCES: Regex = Regex::new(r"\d+ differences").unwrap();
        static ref RE_SUCCESS_SOME: Regex = Regex::new(r"success \d+").unwrap();
        static ref RE_SOME_PROCESSED: Regex = Regex::new(r"\d+ processed").unwrap();
        static ref RE_LEDGER_SOME: Regex = Regex::new(r"Ledger \d+").unwrap();
        static ref RE_ACCOUNT_SOME: Regex = Regex::new(r"r[a-zA-Z0-9]{25,35}").unwrap();
        static ref RE_DONE_COMPLETE: Regex = Regex::new(r"complete \d+").unwrap();
        static ref RE_FETCH_PACK: Regex = Regex::new(r"pack for \d+").unwrap();
        static ref RE_NUM_OUT_OF: Regex = Regex::new(r"\d+ out of \d+").unwrap();
        static ref RE_BOOKS_FOUND: Regex = Regex::new(r"\d+ books found").unwrap();
        static ref RE_TIMEOUTS_SOME: Regex = Regex::new(r"timeouts:\d+").unwrap();
        static ref RE_STATUS_OTHER_THAN: Regex = Regex::new(r"Status other than -?\d+").unwrap();
        static ref RE_THRESH_SOME: Regex = Regex::new(r"Thresh:\d+").unwrap();
        static ref RE_SAVE_FOR: Regex = Regex::new(r"save for \d+").unwrap();
        static ref RE_LEDGER_OBJ: Regex = Regex::new(r"\{.+acquired.+}").unwrap();
        static ref RE_SOME_FAILED_AND_SOME: Regex = Regex::new(r"\d+ failed and \d+").unwrap();
        static ref RE_NODE_COUNT_SOME: Regex = Regex::new(r"Node count \(\d+\)").unwrap();
        static ref RE_AMOUNT_CURRENCY: Regex = Regex::new(r"\d+(\.\d+)?/[A-Z]{3}").unwrap();
    }

    // replace base-16 hashes of length 64 (e.g.: 58B57FBEF009EB802DA44B7B35E362DA33648FCD2FE3C3DA235C54EFC8A082A8)
    let msg_sanitized = &RE_BASE_16.replace_all(msg, "#some-base-16-hash");
    // replace alpha numerical ids of length 52 (e.g.: nHBe4vqSAzjpPRLKwSFzRFtmvzXaf5wPPmuVrQCAoJoS1zskgDA4)
    let msg_sanitized = &RE_ALPHA_NUM_ID.replace_all(msg_sanitized, "#some-id");
    // replace ip addresses
    let msg_sanitized = &RE_IP.replace_all(msg_sanitized, "#some-ip");
    // replace numbers with '#' prefix (e.g.: #5334)
    let msg_sanitized = &RE_HASH_NUM.replace_all(msg_sanitized, "#some-num");
    // replace amount/currency pairs (e.g.: 36981682439/XRP)
    let msg_sanitized = &RE_AMOUNT_CURRENCY.replace_all(msg_sanitized, "#amount/#currency");
    let msg_sanitized = &RE_SOME_PEER.replace_all(msg_sanitized, "Peer #some-peer-node votes");
    let msg_sanitized = &RE_SOME_PEER_NOW.replace_all(msg_sanitized, "Peer #some-peer-node now");
    let msg_sanitized = &RE_SOME_PEER_HAS.replace_all(msg_sanitized, "#some-peer-node has");
    let msg_sanitized = &RE_SOME_PEER_VOTES.replace_all(msg_sanitized, "votes #some-vote on");
    let msg_sanitized = &RE_WEIGHT.replace_all(msg_sanitized, "#some-weight");
    let msg_sanitized = &RE_PERCENT.replace_all(msg_sanitized, "#some-percent");
    let msg_sanitized = &RE_VOTES.replace_all(msg_sanitized, "#some-votes time votes");
    let msg_sanitized = &RE_PARTICIPANTS.replace_all(msg_sanitized, "#some-participants");
    let msg_sanitized = &RE_LEDGER_ID.replace_all(msg_sanitized, ": #some-ledger-id <=");
    let msg_sanitized = &RE_LEDGER_ID_TRAIL.replace_all(msg_sanitized, "<= #some-ledger-id");
    let msg_sanitized =
        &RE_ADVANCE_LEDGER_ID.replace_all(msg_sanitized, "#some-ledger-id >= #validations");
    let msg_sanitized = &RE_LEDGER_JSON_LOG.replace_all(msg_sanitized, "LEDGER_STATUS_JSON_LOG");
    let msg_sanitized = &RE_PROPOSERS.replace_all(msg_sanitized, "Proposers:#some-proposers");
    let msg_sanitized = &RE_THRESH_WEIGHT.replace_all(msg_sanitized, "#some-needweight");
    let msg_sanitized = &RE_THRESH_VOTE.replace_all(msg_sanitized, "#some-thresh-vote");
    let msg_sanitized = &RE_THRESH_CONSENSUS.replace_all(msg_sanitized, "#some-thresh-consensus");
    let msg_sanitized = &RE_OFFSET_ESTIMATE.replace_all(
        msg_sanitized,
        "is estimated at #some-offset (#some-closecount)",
    );
    let msg_sanitized = &RE_NUM_NODES.replace_all(msg_sanitized, "#num nodes");
    let msg_sanitized = &RE_BRACKETS_NUM.replace_all(msg_sanitized, "");
    let msg_sanitized = &RE_SEQ_NUM.replace_all(msg_sanitized, "seq=#");
    let msg_sanitized =
        &RE_LEDGER_TIMEOUTS.replace_all(msg_sanitized, "# timeouts for ledger #some-ledger-id");
    let msg_sanitized =
        &RE_MISSING_NODE.replace_all(msg_sanitized, "Missing node in #some-ledger-id");
    let msg_sanitized = &RE_SOME_TASKS.replace_all(msg_sanitized, "#some-tasks tasks");
    let msg_sanitized = &RE_SOME_JOBS.replace_all(msg_sanitized, "#some-jobs jobs");
    let msg_sanitized = &RE_SOME_ITEMS.replace_all(msg_sanitized, "#some-items items");
    let msg_sanitized = &RE_SOME_OF_SOME.replace_all(msg_sanitized, "#some of #some listed");
    let msg_sanitized = &RE_SOME_OF.replace_all(msg_sanitized, "#some of");
    let msg_sanitized = &RE_OF_SOME_FOR.replace_all(msg_sanitized, "of #some for");
    let msg_sanitized = &RE_SOME_SOME_ID.replace_all(msg_sanitized, "#some:#some-id");
    let msg_sanitized = &RE_SOME_TRUSTED.replace_all(msg_sanitized, "#some trusted");
    let msg_sanitized = &RE_SOME_ADDED.replace_all(msg_sanitized, "#some added");
    let msg_sanitized = &RE_SOME_REMOVED.replace_all(msg_sanitized, "#some removed");
    let msg_sanitized = &RE_SOME_GOOD_NUM.replace_all(msg_sanitized, "good:#some-good-num");
    let msg_sanitized = &RE_SOME_DUPE_NUM.replace_all(msg_sanitized, "dupe:#some-dupe-num");
    let msg_sanitized = &RE_SOME_SRC.replace_all(msg_sanitized, "src=#some-src-num");
    let msg_sanitized = &RE_SOME_FROM.replace_all(msg_sanitized, "from #some_number");
    let msg_sanitized = &RE_SOME_N.replace_all(msg_sanitized, "n=#some-num");
    let msg_sanitized = &RE_SOME_TRANSACTIONS.replace_all(msg_sanitized, "#some transactions");
    let msg_sanitized = &RE_SOME_CHANGES.replace_all(msg_sanitized, "#some changes");
    let msg_sanitized = &RE_SOME_AND.replace_all(msg_sanitized, "#some and");
    let msg_sanitized = &RE_SOME_BEGINS.replace_all(msg_sanitized, "#some begins");
    let msg_sanitized = &RE_SOME_COMPLETED.replace_all(msg_sanitized, "#some completed");
    let msg_sanitized = &RE_SOME_ACCOUNTS.replace_all(msg_sanitized, "#some accounts");
    let msg_sanitized = &RE_IS_SOME_NL.replace_all(msg_sanitized, "is #some");
    let msg_sanitized = &RE_TO_SOME_NL.replace_all(msg_sanitized, "to #some");
    let msg_sanitized = &RE_HASH_COLON_SOME.replace_all(msg_sanitized, "#some-base-16-hash:#some");
    let msg_sanitized =
        &RE_SOME_BRANCH_SUPPORT_OBJECT.replace_all(msg_sanitized, "#some-branch-support-object");
    let msg_sanitized =
        &RE_AGREE_DISAGREE.replace_all(msg_sanitized, "agree=#some, disagree=#some");
    let msg_sanitized = &RE_SOME_CONSENSUS_DBG.replace_all(msg_sanitized, "(#truncated)");
    let msg_sanitized = &RE_REPORT_SOME_PROP.replace_all(
        msg_sanitized,
        "Prop=#some val=#some corLCL=#some fail=#some",
    );
    let msg_sanitized = &RE_PROGRESS_SOME.replace_all(msg_sanitized, "progress(#some)");
    let msg_sanitized =
        &RE_TIMEOUT_SOME.replace_all(msg_sanitized, "Timeout(#some) pc=#some acquiring");
    let msg_sanitized = &RE_HELD_SOME.replace_all(msg_sanitized, "held: #some");
    let msg_sanitized =
        &RE_BALANCE_SOME.replace_all(msg_sanitized, "Balance: #some-value/#currency");
    let msg_sanitized =
        &RE_OFFER_OUT.replace_all(msg_sanitized, "Offer out: #some-value/#currency");
    let msg_sanitized =
        &RE_OFFER_IN_SOME_ISSUER.replace_all(msg_sanitized, "Offer in: #some-value/#currency");
    let msg_sanitized = &RE_CROSSING_AS_SOME.replace_all(msg_sanitized, "Crossing as: #some-id");
    let msg_sanitized = &RE_ATTEMPTING_CROSS_ONE.replace_all(
        msg_sanitized,
        "Attempting cross: #some-account/#currency -> #currency",
    );
    let msg_sanitized = &RE_ATTEMPTING_CROSS_TWO.replace_all(
        msg_sanitized,
        "Attempting cross: #currency -> #some-account/#currency",
    );
    let msg_sanitized = &RE_ATTEMPTING_CROSS_DOUBLE.replace_all(
        msg_sanitized,
        "Attempting cross: #some-account/#currency -> #some-account/#currency",
    );
    // let msg_sanitized =
    //     &RE_FINAL_RESULT.replace_all(msg_sanitized, "final result: #some");
    let msg_sanitized = &RE_ORDER_SOME_VALUE.replace_all(msg_sanitized, "order #some-value");
    let msg_sanitized =
        &RE_HAS_SOME_SOME_REQUIRED.replace_all(msg_sanitized, "has #some, #some required");
    let msg_sanitized = &RE_SEQ_SOME.replace_all(msg_sanitized, "seq #some:");
    let msg_sanitized = &RE_SOME_NAYS_OBJECT.replace_all(msg_sanitized, "{truncated}");
    let msg_sanitized = &RE_SOME_DIFFERENCES.replace_all(msg_sanitized, "#some differences");
    let msg_sanitized = &RE_SUCCESS_SOME.replace_all(msg_sanitized, "success #some");
    let msg_sanitized = &RE_SOME_PROCESSED.replace_all(msg_sanitized, "#some processed");
    let msg_sanitized = &RE_ACCOUNT_SOME.replace_all(msg_sanitized, "#some-account");
    let msg_sanitized = &RE_LEDGER_SOME.replace_all(msg_sanitized, "Ledger #some");
    let msg_sanitized = &RE_DONE_COMPLETE.replace_all(msg_sanitized, "complete #some-num");
    // replace ledger close times
    let msg_sanitized = &RE_LEDGER_CLOSE_TIME.replace_all(msg_sanitized, "#some-ledger-close-time");
    let msg_sanitized = &RE_FETCH_PACK.replace_all(msg_sanitized, "pack for #some-obj");
    let msg_sanitized = &RE_NUM_OUT_OF.replace_all(msg_sanitized, "#some out of #some");
    let msg_sanitized = &RE_BOOKS_FOUND.replace_all(msg_sanitized, "#some books found");
    let msg_sanitized = &RE_TIMEOUTS_SOME.replace_all(msg_sanitized, "timeouts:#some");
    let msg_sanitized = &RE_STATUS_OTHER_THAN.replace_all(msg_sanitized, "Status other than #some");
    let msg_sanitized = &RE_THRESH_SOME.replace_all(msg_sanitized, "Thresh:#some");
    let msg_sanitized = &RE_SAVE_FOR.replace_all(msg_sanitized, "pack for #some");
    let msg_sanitized = &RE_LEDGER_OBJ.replace_all(msg_sanitized, "{truncated}");
    let msg_sanitized =
        &RE_SOME_FAILED_AND_SOME.replace_all(msg_sanitized, "#some failed and #some");
    let msg_sanitized = &RE_NODE_COUNT_SOME.replace_all(msg_sanitized, "Node count (#some)");

    return msg_sanitized.to_string();
}
