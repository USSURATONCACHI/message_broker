@0xd39a69b7918524e0;


interface Echo { 
    struct EchoRequest {
        message @0 :Text;
    }

    struct EchoResponse {
        message @0 :Text;
    }

    struct Ping {
        message @0 :Text;
    }

    echo @0 (request :EchoRequest) -> (reply :EchoResponse);
    # ping @1 () -> (reply :Ping);
}