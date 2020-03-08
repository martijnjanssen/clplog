import re
import sys

log_entering_consensus = "LedgerConsensus:NFO Entering consensus process"


def main():
    if len(sys.argv) < 2:
        raise Exception("missing argument for log file")

    filename = sys.argv[1]

    with open(filename) as file:
        parse_file(file)


def parse_file(file):
    # count distinct logs
    log_id_counter = 0
    # map log_string -> log_id
    log_id_map = dict()
    # list index log_id -> log_string
    log_list = list()
    # list index log_id -> count
    log_counts = list()
    # list index consensus_round -> log_id sequence (e.g.: [[1,2,3,4],[1,3,4,5]])
    all_log_sequence = list()

    # stream over lines in file so we don't have to load the whole file into memory
    for line_number, line in enumerate(file):
        # split line in [date, time, msg]
        l = line.split(" ", 2)
        if len(l) != 3 or not line.startswith("2020"):
            continue
        msg = l[2]

        # start of consensus round
        if msg.startswith(log_entering_consensus):
            all_log_sequence.append(list())

        # skip if not yet in consensus round
        if len(all_log_sequence) == 0:
            continue

        # skip non-informative lines for protocol
        if msg.startswith("Application:NFO") or msg.startswith("Peer:"):
            continue

        # replace base-16 hashes of length 64 (e.g.: 58B57FBEF009EB802DA44B7B35E362DA33648FCD2FE3C3DA235C54EFC8A082A8)
        msg = re.sub(r"[0-9A-F]{64}", "some-base-16-hash", msg)
        # replace alpha numerical ids of length 52 (e.g.: nHBe4vqSAzjpPRLKwSFzRFtmvzXaf5wPPmuVrQCAoJoS1zskgDA4)
        msg = re.sub(r"[A-Za-z0-9]{52}", "some-id", msg)
        # replace ip addresses
        msg = re.sub(r"(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?", "some-ip", msg)
        # replace numbers with '#' prefix (e.g.: #5334)
        msg = re.sub(r"#\d+", "#some-num", msg)

        # if you pipe the output of the script (e.g. `python main.py debug.log | less`)
        # you can see how the resulting lines look after filtering
        if print_lines:
            print(l[0], l[1], msg.strip())

        # if this is a new unique log line
        if msg not in log_id_map:
            # add it to the map
            log_id_map[msg] = log_id_counter
            # and to the list
            log_list.append(msg)
            # and give it a count
            log_counts.append(0)
            # and increase the unique log counter
            log_id_counter += 1

        # get the log id
        log_id = log_id_map[msg]
        # increase the count
        log_counts[log_id] += 1
        # append the id to the current sequence
        current_sequence = len(all_log_sequence) - 1
        all_log_sequence[current_sequence].append(log_id)

        # if log_id_counter == 10000:
        #     break

    for i, l in enumerate(all_log_sequence):
        print(i, len(l))
    print(len(log_list))
    print(line_number)

    #TODO create file in following format:
    # first line: <len(all_log_sequence)> <len(log_list)>
    # for each sequence: <accepting?1:0> <len(sequence)> <log_id_1> <log_id_2> ...


if __name__ == "__main__":
    print_lines = not sys.stdout.isatty()
    main()
