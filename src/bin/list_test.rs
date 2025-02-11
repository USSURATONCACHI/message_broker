use std::sync::atomic::Ordering;
use std::sync::Arc;

use broker::concurrent_list::{ChunkRef, ConcurrentList, APPEND_LOCKS, APPEND_MISSES, READ_LOCKS, TOTAL_APPENDS, TOTAL_ELEMENTS_WRITTEN, TOTAL_READS};

fn print_array(reader_id: usize, array: ChunkRef<String>) -> std::io::Result<()> {
    let mut total_read = 0usize;
    for elem in array {
        assert!(elem.starts_with("Phrase "));
        total_read += 1;
    }
    println!("Reader {reader_id}, {total_read} elements was read");

    Ok(())
}

fn push_to_array(writer_id: usize, mut array: ChunkRef<String>) {
    for i in 0..100 {
        array.push(format!("Phrase {} - {}", writer_id, i));
        TOTAL_ELEMENTS_WRITTEN.fetch_add(1, Ordering::Relaxed);
        // std::thread::sleep(Duration::from_millis(4));
    }
    println!("Writer {writer_id} done.");
}


fn main() {
    let mut all_threads = Vec::new();

    // Create array
    let array = Arc::new(ConcurrentList::<String>::new(256));
    
    // Add N writers
    for i in 0..10 {
        let array = array.reference();
        let t = std::thread::spawn(move || push_to_array(i, array));
        all_threads.push(t);
    }

    // Add N readers
    for i in 0..10 {
        let array = array.reference();
        let t = std::thread::spawn(move || print_array(i, array).unwrap());
        all_threads.push(t);
    }

    // Wait on threads    
    for t in all_threads {
        match t.join() {
            Ok(_) => {}
            Err(e) => {
                println!("Thread failed: {:?}", e);
            }
        }
    }

    let reads = TOTAL_READS.load(Ordering::Relaxed);
    let read_locks = READ_LOCKS.load(Ordering::Relaxed);
    let appends = TOTAL_APPENDS.load(Ordering::Relaxed);
    let append_locks = APPEND_LOCKS.load(Ordering::Relaxed);
    let append_misses = APPEND_MISSES.load(Ordering::Relaxed);

    println!("Total reads: {reads}, read locks: {read_locks} ({}%)", read_locks as f64 / reads as f64 * 100.0);
    println!("Total appends: {appends}, append locks: {append_locks} ({}%)", append_locks as f64 / appends as f64 * 100.0);
    
    let nodes_count = array.nodes_count();
    println!("Allocated nodes: {nodes_count}");
    println!("Append allocation misses: {append_misses} ({}%) (this many nodes were allocated and instantly deallocated)", append_misses as f64 / (append_misses + nodes_count) as f64 * 100.0);
    
    let total_written = TOTAL_ELEMENTS_WRITTEN.load(Ordering::Relaxed);
    let len = array.len();
    println!("Total written: {total_written}. Resulting length: {len}. Elements lost: {} ({}%)", total_written - len, (total_written - len) as f64 / total_written as f64 * 100.0);
}
