@0xd6726b81b5640564;

struct Uuid {
    lower @0 :UInt64;
    upper @1 :UInt64;
}

struct Timestamp {
    seconds @0 :Int64;
    nanos @1 :UInt32;
}

struct Result(T, E) {
    union {
        ok @0 :T;
        err @1 :E;
    }
}

struct None {
    
}