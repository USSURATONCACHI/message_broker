@0x96b8f72124f7c404;

interface AuthService {
    login @0 (username :Text) -> ();
    logout @1 () -> ();
}
