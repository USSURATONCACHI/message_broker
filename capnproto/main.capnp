@0xd39a69b7918524e0;

using Auth = import "auth.capnp";
using Topic = import "topic.capnp";
using Message = import "message.capnp";

interface RootService { 
    auth @0 () -> (service :Auth.AuthService);
    topic @1 () -> (service :Topic.TopicService);
    message @2 () -> (service :Message.MessageService);
}