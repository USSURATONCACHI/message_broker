@0xdb082feea6666c8d;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Timestamp;
using Util.AuthResult;
using Util.Authorized;
using Util.None;

struct Topic {
    uuid @0 :Uuid;
    name @1 :Text;
    ownerId @2 :Uuid;
    createdAt @3 :Timestamp;
}

interface TopicService {
    struct Error {
        union {
            notFound @0 :Void;
            alreadyExists @1 :Void;
        }
    }

    createTopic @0 (name :Text) -> (topic :AuthResult(Topic, Error));

    getTopic @1 (topicId :Uuid) -> (topic :AuthResult(Topic, Error));
    getAllTopics @2 () -> (topics :Authorized(List(Topic)));

    updateTopic @3 (topicId :Uuid, name :Text) -> (topic :AuthResult(Topic, Error));
    deleteTopic @4 (topicId :Uuid) -> (result :AuthResult(None, Error));
}
