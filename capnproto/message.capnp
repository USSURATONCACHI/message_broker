@0x8dca14b2a5146de8;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Timestamp;
using Util.Result;
using Util.None;
using Util.Option;

struct Message {
    uuid @0 :Uuid;
    authorName @1 :Text;
    content @2 :Text;
    timestamp @3 :Timestamp;
    topicUuid @4 :Uuid;
    key @5 :Option(Text);
}

interface MessageService {
    struct Error {
        union {
            entityDoesNotExist @0 :Void;
            invalidContent @1 :Void;
        }
    }

    postMessage @0 (topicId :Uuid, content :Text) -> (message :Result(Message, Error));
    deleteMessage @1 (messageId :Uuid)  -> (result :Result(None, Error));

    getMessagesSync @2 (topicId :Uuid) -> (messages :Result(List(Message), Error));

    subscribe @3 (topicId :Uuid, receiver :MessageReceiver) -> (messages :Result(ReverseMessageIterator, Error));
    unsubscribe @4 (topicId :Uuid, receiver :MessageReceiver) -> ();
}

interface ReverseMessageIterator {
    next @0 (count :UInt32) -> (messages :List(Message));
    stop @1 () -> ();
}

interface MessageReceiver {
    receive @0 (message :Message) -> stream;
}