@0xfe159ddc49b4a4d1;

interface Echo { 
    struct EchoRequest {
        message @0 :Text;
    }

    struct EchoResponse {
        message @0 :Text;
    }

    echo @0 (request :EchoRequest) -> (reply :EchoResponse);
    subscribeToPings @1 (receiver :PingReceiver) -> ();
}

interface PingReceiver {
    ping @0 (seq :UInt64) -> ();
}
