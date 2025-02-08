@0x8dca14b2a5146de8;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Timestamp;
using Util.AuthResult;
using Util.Authorized;
using Util.None;

struct Message {
    uuid @0 :Uuid;
    authorUuid @1 :Uuid;
    content @2 :Text;
    timestamp @3 :Timestamp;
}

interface MessageService {
    struct Error {
        union {
            topicDoesntExist @0 :Void;
            invalidContent @1 :Void;
        }
    }

    postMessage @0 (topicId :Uuid, content :Text) -> (message :AuthResult(Message, Error));
    
    deleteMessage @1 (messageId :Uuid)  -> (result :AuthResult(None, Error));

    getMessages @2 (topicId :Uuid) -> (messages :AuthResult(List(Message), Error));
}

# Subscription Management
interface SubscriptionService {
    struct Error {
        union {
            topicDoesntExist @0 :Void;
            reserved @1 :Void;
        }
    }

    subscribe @0 (topicId :Uuid) -> (result :AuthResult(None, Error));
    unsubscribe @1 (topicId :Uuid) -> (result :AuthResult(None, Error));
    listSubscriptions @2 () -> (topicIds :Authorized(List(Uuid)));
    
    # SSE-style live updates
    subscribeToLiveMessages @3 (receiver :MessageReceiver) -> (result :Authorized(None));
}

interface MessageReceiver {
    newMessage @0 (message :Message) -> ();
}