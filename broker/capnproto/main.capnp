@0xd39a69b7918524e0;

using Auth = import "auth.capnp";
using Topic = import "topic.capnp";
using Message = import "message.capnp";
using Echo = import "echo.capnp";

interface RootService { 
    auth @0 () -> (service :Auth.AuthService);
    echo @1 () -> (service :Echo.Echo);
}