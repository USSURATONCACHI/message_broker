@0xdb082feea6666c8d;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Timestamp;
using Util.Result;
using Util.None;

struct Retention {
    union {
        none @0 :Void;
        minutes @1 :Float64;
    }
}

struct Topic {
    uuid @0 :Uuid;
    name @1 :Text;
    ownerUsername @2 :Text;
    createdAt @3 :Timestamp;
    retention @4 :Retention;
}

interface TopicService {
    struct Error {
        union {
            notFound @0 :Void;
            alreadyExists @1 :Void;
        }
    }

    createTopic @0 (name :Text) -> (topic :Result(Topic, Error));

    getTopic @1 (topicId :Uuid) -> (topic :Result(Topic, Error));
    getAllTopics @2 () -> (topics :List(Topic));

    updateTopic @3 (topicId :Uuid, name :Text, retention :Retention) -> (topic :Result(Topic, Error));
    deleteTopic @4 (topicId :Uuid) -> (result :Result(None, Error));
}
