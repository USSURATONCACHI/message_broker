@0x8dca14b2a5146de8;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Timestamp;
using Util.Result;
using Util.None;

struct Message {
    uuid @0 :Uuid;
    authorName @1 :Text;
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

    postMessage @0 (topicId :Uuid, content :Text) -> (message :Result(Message, Error));
    
    deleteMessage @1 (messageId :Uuid)  -> (result :Result(None, Error));

    getMessagesSync @2 (topicId :Uuid) -> (messages :Result(List(Message), Error));

    subscribe @3 (topicId :Uuid, receiver :MessageReceiver) -> (messages :Result(None, Error));
    unsubscribe @4 (topicId :Uuid, receiver :MessageReceiver) -> ();
}

interface MessageReceiver {
    receive @0 (message :Message, topic :Uuid) -> stream;
}

# # Subscription Management
# interface SubscriptionService {
#     struct Error {
#         union {
#             topicDoesntExist @0 :Void;
#             reserved @1 :Void;
#         }
#     }

#     subscribe @0 (topicId :Uuid) -> (result :Result(None, Error));
#     unsubscribe @1 (topicId :Uuid) -> (result :Result(None, Error));
#     listSubscriptions @2 () -> (topicIds :List(Uuid));
    
#     # SSE-style live updates
#     subscribeToLiveMessages @3 (receiver :MessageReceiver) -> ();
# }

# interface MessageReceiver {
#     newMessage @0 (message :Message) -> ();
# }