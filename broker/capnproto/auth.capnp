@0x96b8f72124f7c404;

using Util = import "util.capnp";
using Util.Uuid;
using Util.Result;
using Util.AuthResult;
using Util.Authorized;
using Util.None;

struct User {
    uuid @0 :Uuid;
    username @1 :Text;
}

struct Token {
    union {
        none @0 :Void;
        jwt @1 :Data;
        # future: apiKey @2 :Text;
    }
}

interface AuthService {
    struct SuccessfulLogin {
        token @0 :Token;
        user @1 :User;
    }
    struct Error {
        union {
            invalidUsername @0 :Void;
            usernameAlreadyTaken @1 :Void;
            invalidPassword @2 :Void;
        }
    }

    register @0 (username :Text, password :Text) -> (login :Result(SuccessfulLogin, Error));

    login @1 (username :Text, password :Text) -> (login :Authorized(SuccessfulLogin));

    unlogin @2 (token :Token) -> (result :Authorized(None));
    
    getCurrentUser @3 (token :Token) -> (user :Authorized(User));
    getAllUsers @4 (token :Token) -> (user :Authorized(List(User)));
}
