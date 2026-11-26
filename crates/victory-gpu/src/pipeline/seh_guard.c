#include <windows.h>

typedef void (*victory_worker_fn)(void* ctx);

int victory_seh_call(victory_worker_fn func, void* ctx) {
    __try {
        func(ctx);
        return 1;
    } __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        return 0;
    }
}
