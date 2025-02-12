N=$1

cleanup() {
    echo "Interrupt received! Killing all subprocesses..."
    kill -9 0
    exit 1
}
trap cleanup SIGINT


echo "Spawning ${N} clients. You sure?"
read

cargo build --release --bin client

run() {
    for i in $(seq 1 $N); do
        echo "Spawning process $i"
        cat hello\ world.txt | ./target/release/client 127.0.0.1:8080 "Сметанси $i" >/dev/null &
    done
    echo "Waiting for end"
    wait
    echo "All processes finished"
}

time run